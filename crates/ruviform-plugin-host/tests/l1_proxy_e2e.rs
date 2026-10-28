//! Control-plane E2E for the `pub(crate)` L1 out-of-process instance
//! machinery (Issue #76 task 3 — `docs/tasks/l1-proxy-control-plane.md`,
//! oracle `docs/test-specs/plugin-l1-proxy-contract-tests.md`).
//!
//! `mod l1` is `pub(crate)` inside the library and cannot be reached
//! through the public API until the `PluginHost` composite lands, so
//! this harness includes the module by `#[path]` — the same trick
//! `ruviform-math` benches use for `tests/simd_poc`. The constraint it
//! places on `src/l1/` (no `crate::` paths) is documented at the top
//! of `l1/mod.rs`.
//!
//! The child fixture is `src/bin/l1-test-worker.rs` via
//! `CARGO_BIN_EXE_l1-test-worker` — kept inside this crate; no
//! cross-package bin resolution.

#[allow(dead_code)]
#[path = "../src/l1/mod.rs"]
mod l1;

use std::ffi::OsString;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{MediaTime, TransportStatus};
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    Event, EventKind, EventList, GuiParentToken, HostError, HostInput, HostOutput, LifecycleError,
    LifecycleEvent, LifecycleState, ParamEvent, ParamId, ParamValue, PluginDescriptor,
    PrepareError, ProcessConfig, ProcessResult, ProcessStatus, TransportInfo,
};

use l1::{ChildExit, L1Child, L1Instance, SpawnSpec};

/// `assert_no_alloc` arms a thread-local forbid flag through the
/// global allocator — required exactly once per test binary (the
/// `rt_l1_*` contract tests).
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC_DISABLER: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

/// The descriptor `l1-test-worker` reports for `Scan` — the
/// `SpawnSpec` oracle must field-match it.
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// How long tests poll for an asynchronous `Failed` notification —
/// crash convergence needs at most a `DRIVER_POLL_INTERVAL` (200 µs)
/// plus scheduling; 10 s is generous headroom for loaded CI hosts.
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

fn spec(args: &[&str]) -> SpawnSpec {
    let mut spec = SpawnSpec::new(
        WORKER_DESCRIPTOR,
        OsString::from(env!("CARGO_BIN_EXE_l1-test-worker")),
    );
    spec.args = args.iter().map(OsString::from).collect();
    spec
}

/// A spec with a short `command_timeout` for the timeout paths —
/// the defined default is 10 s, far too long for a hang test.
fn fast_timeout_spec(args: &[&str]) -> SpawnSpec {
    let mut spec = spec(args);
    spec.command_timeout = Duration::from_millis(500);
    spec
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
}

/// Polls `try_recv_notification` until the `Failed` notification
/// arrives or `NOTIFY_TIMEOUT` elapses.
fn wait_notification(instance: &L1Instance) -> Option<ChildExit> {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    loop {
        if let Some(exit) = instance.try_recv_notification() {
            return Some(exit);
        }
        assert!(
            Instant::now() < deadline,
            "Failed notification never arrived"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_failed(instance: &L1Instance) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while instance.lifecycle_state() != LifecycleState::Failed {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Command parity (spec AC1): the L0 command series runs
/// child-side and returns identical reply/error semantics.
#[test]
fn command_parity_lifecycle_and_state() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    assert_eq!(instance.lifecycle_state(), LifecycleState::Created);

    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Initialized);

    assert_eq!(
        instance.execute(wire::Command::Activate { config: config() }),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Activated);
    // The armed ring reports the lookahead horizon — `(depth − 1)
    // × armed frame_capacity` = 2 × 512.
    assert_eq!(instance.latency_frames(), Some(1024));

    assert_eq!(
        instance.execute(wire::Command::StartProcessing),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);

    let saved = instance.execute(wire::Command::SaveState).unwrap();
    let wire::Reply::State { bytes } = saved else {
        panic!("unexpected reply");
    };
    assert_eq!(bytes, [0x11, 0x22, 0x33, 0x44]);

    assert_eq!(
        instance.execute(wire::Command::RestoreState {
            bytes: vec![0xAA, 0xBB]
        }),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(
        instance.execute(wire::Command::SaveState),
        Ok(wire::Reply::State {
            bytes: vec![0xAA, 0xBB]
        })
    );

    assert_eq!(
        instance.execute(wire::Command::StopProcessing),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Activated);
    assert_eq!(
        instance.execute(wire::Command::Deactivate),
        Ok(wire::Reply::Ok)
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Initialized);
    // The armed ring survives `Deactivate` (`staging` is never
    // cleared) but the public query gates on the mirror — `None`,
    // L0 parity, not the stale horizon.
    assert_eq!(instance.latency_frames(), None);

    // A post-handshake `Scan` still reports the child's descriptors.
    match instance.execute(wire::Command::Scan) {
        Ok(wire::Reply::Scanned(descriptors)) => {
            assert!(descriptors.iter().any(|d| d.matches(&WORKER_DESCRIPTOR)));
        }
        other => panic!("unexpected reply: {other:?}"),
    }

    instance.destroy();
    assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);
    // Clean teardown never notifies.
    assert!(instance.try_recv_notification().is_none());
}

/// Illegal lifecycle transitions return the child-side
/// `Err(HostError::Lifecycle(_))` — identical semantics to L0.
#[test]
fn illegal_transitions_match_l0_semantics() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();

    // Activate before Initialize.
    assert!(matches!(
        instance.execute(wire::Command::Activate { config: config() }),
        Err(HostError::Lifecycle(_))
    ));

    instance.execute(wire::Command::Initialize).unwrap();
    instance
        .execute(wire::Command::Activate { config: config() })
        .unwrap();
    instance.execute(wire::Command::StartProcessing).unwrap();

    // Deactivate while Processing — rejected, machine untouched.
    assert!(matches!(
        instance.execute(wire::Command::Deactivate),
        Err(HostError::Lifecycle(_))
    ));
    assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);

    // Double Initialize once back at Initialized.
    instance.execute(wire::Command::StopProcessing).unwrap();
    instance.execute(wire::Command::Deactivate).unwrap();
    assert!(matches!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Lifecycle(_))
    ));

    instance.destroy();
}

/// Crash contract (spec AC4, EOF/`try_wait` paths): the worker dies
/// on the 4th received message — the handshake `Scan` is message 1,
/// `CreateInstance` is 2 — so `Activate`'s pending waiter is woken
/// `Err(Ipc)`, the instance converges `Failed` with exactly one
/// notification, and every later command except `Destroy` is
/// rejected.
#[test]
fn crash_transitions_failed_and_notifies() {
    let mut instance = L1Instance::spawn(&spec(&["--crash-after-msgs=4"])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    // The pending command is woken `Ipc` by the convergence.
    assert_eq!(
        instance.execute(wire::Command::Activate { config: config() }),
        Err(HostError::Ipc)
    );

    wait_for_failed(&instance);
    let exit = wait_notification(&instance).expect("notification missing");
    assert_eq!(exit.code, Some(3));

    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Failed)
    );
    assert_eq!(
        instance.execute(wire::Command::SaveState),
        Err(HostError::Failed)
    );
    // `Failed` precedes the `Unsupported` GUI rejection.
    assert_eq!(
        instance.execute(wire::Command::OpenGui {
            parent: GuiParentToken::new(0)
        }),
        Err(HostError::Failed)
    );

    // `Destroy` is the defined rejection's escape — `Ok` (kill/reap
    // already handled by the driver).
    instance.destroy();
    assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);

    // The notification fires exactly once.
    assert!(instance.try_recv_notification().is_none());
}

/// A voluntary `exit(0)` while live is still a protocol violation —
/// `Failed`, not clean teardown.
#[test]
fn voluntary_exit_zero_is_failure() {
    let mut instance =
        L1Instance::spawn(&spec(&["--crash-after-msgs=3", "--crash-exit-code=0"])).unwrap();

    // Message 3 (`Initialize` — after `Scan` and `CreateInstance`)
    // kills the worker mid-command.
    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Ipc)
    );

    wait_for_failed(&instance);
    let exit = wait_notification(&instance).expect("notification missing");
    assert_eq!(exit.code, Some(0));
    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Failed)
    );
    instance.destroy();
}

/// The driver-side `try_wait` path: `kill()` drives the same
/// convergence as a spontaneous crash.
#[test]
fn kill_transitions_failed_and_notifies() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    instance.kill();

    wait_for_failed(&instance);
    wait_notification(&instance).expect("notification missing");
    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Failed)
    );
    instance.destroy();
    // No automatic restart — repeated commands keep failing.
    assert_eq!(
        instance.execute(wire::Command::SaveState),
        Err(HostError::Failed)
    );
}

/// A child that stops replying converges on `command_timeout`:
/// `Err(Ipc)` plus `Failed` — the driver then kills it (defined
/// rejection table: timeout ⇒ `Ipc` + `Failed`).
#[test]
fn hang_converges_on_command_timeout() {
    let mut instance = L1Instance::spawn(&fast_timeout_spec(&["--hang-after-msgs=2"])).unwrap();

    // `Scan` and `CreateInstance` (the handshake) were messages 1–2 —
    // `Initialize` is never answered.
    assert_eq!(
        instance.execute(wire::Command::Initialize),
        Err(HostError::Ipc)
    );

    wait_for_failed(&instance);
    wait_notification(&instance).expect("notification missing");
    // Destroy on the still-running hung child stays bounded — the
    // driver kills it.
    instance.destroy();
    assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);
}

/// Descriptor mismatch at handshake: the child's `Scanned` does not
/// field-match `spec.descriptor` → `Err(Ipc)` + kill/reap.
#[test]
fn handshake_descriptor_mismatch() {
    let mut spec = spec(&[]);
    spec.descriptor = PluginDescriptor {
        name: "Not The Worker",
        ..WORKER_DESCRIPTOR
    };
    assert_eq!(L1Instance::spawn(&spec).map(|_| ()), Err(HostError::Ipc));
}

/// `Hello.version != WIRE_VERSION` → handshake failure → `Err(Ipc)`.
#[test]
fn handshake_version_mismatch() {
    assert_eq!(
        L1Instance::spawn(&spec(&["--hello-version=99"])).map(|_| ()),
        Err(HostError::Ipc)
    );
}

/// `Hello` arrives but `Scan` is never answered — the handshake
/// `command_timeout` path → `Err(Ipc)` + kill/reap.
#[test]
fn handshake_scan_timeout() {
    assert_eq!(
        L1Instance::spawn(&fast_timeout_spec(&["--hang-after-msgs=0"])).map(|_| ()),
        Err(HostError::Ipc)
    );
}

/// The executable itself failing to spawn is `Err(Ipc)`.
#[test]
fn spawn_failure_is_ipc() {
    let mut spec = spec(&[]);
    spec.program = OsString::from("ruviform-l1-definitely-not-a-binary-xyz");
    assert_eq!(L1Instance::spawn(&spec).map(|_| ()), Err(HostError::Ipc));
}

/// `OpenGui`/`CloseGui` are rejected `Unsupported` without touching
/// the wire — the child stays alive and keeps serving commands.
#[test]
fn gui_defined_rejection() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    assert_eq!(
        instance.execute(wire::Command::OpenGui {
            parent: GuiParentToken::new(0)
        }),
        Err(HostError::Unsupported)
    );
    assert_eq!(
        instance.execute(wire::Command::CloseGui),
        Err(HostError::Unsupported)
    );
    // Not forwarded: the child still answers normally.
    assert_eq!(
        instance.execute(wire::Command::SaveState),
        Ok(wire::Reply::State {
            bytes: vec![0x11, 0x22, 0x33, 0x44]
        })
    );
    instance.destroy();
}

/// `Activate` over the wire bound is rejected at the proxy:
/// `Prepare(UnsupportedConfig)`, mirror unchanged.
#[test]
fn activate_over_wire_limit_rejected() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    let over = ProcessConfig::new(
        SampleRate::new(48_000).unwrap(),
        1,
        wire::WIRE_MAX_FRAMES + 1,
    )
    .unwrap();
    assert_eq!(
        instance.execute(wire::Command::Activate { config: over }),
        Err(HostError::Prepare(PrepareError::UnsupportedConfig))
    );
    assert_eq!(instance.lifecycle_state(), LifecycleState::Initialized);

    instance.destroy();
}

/// `Shutdown` + reap on a live instance is the only clean exit —
/// no `Failed`, no notification, idempotent `Destroy`.
#[test]
fn destroy_is_clean_teardown() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    instance.destroy();
    assert_eq!(instance.lifecycle_state(), LifecycleState::Destroyed);
    assert!(instance.try_recv_notification().is_none());
    // Idempotent: a second Destroy is `Ok` and sends nothing.
    instance.destroy();
}

/// Dropping a live instance kills + reaps the child — the bounded
/// fallback teardown path (owner gone → driver `Disconnected` arm).
#[test]
fn drop_reaps_live_child() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();
    drop(instance);
    // Reaching here at all is the assertion — Drop must not hang.
}

/// Wedged-child teardown (A1): after `--stop-reading-after-msgs=3`
/// (`Scan`, `CreateInstance`, `Initialize` all answered) the child
/// never drains stdin again, so a `RestoreState` frame far
/// over the pipe capacity leaves the driver blocked in `write_all`.
/// The `command_timeout` convergence must itself kill the child —
/// that is what turns the wedged write into EPIPE and lets the
/// driver unwind — and destroy/drop afterwards stay bounded.
#[test]
fn wedged_child_write_converges_and_teardown_is_bounded() {
    let mut instance =
        L1Instance::spawn(&fast_timeout_spec(&["--stop-reading-after-msgs=3"])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();

    // ~256 KiB — comfortably past every supported platform's pipe
    // capacity and well under the 16 MiB wire bound.
    let big = vec![0xAB_u8; 256 * 1024];
    assert_eq!(
        instance.execute(wire::Command::RestoreState { bytes: big }),
        Err(HostError::Ipc)
    );

    wait_for_failed(&instance);
    wait_notification(&instance).expect("notification missing");
    let started = Instant::now();
    instance.destroy();
    drop(instance);
    assert!(
        started.elapsed() < NOTIFY_TIMEOUT,
        "teardown blocked on the wedged driver"
    );
}

/// Detached reader (A4): `--leave-heir-after-msgs=2` makes the worker
/// answer `Scan` and `CreateInstance` (the whole handshake), then
/// spawn a stdio-inheriting
/// grandchild and exit — the parent's stdout pipe stays open, so the
/// reader thread never observes EOF. `try_wait` still converges
/// `Failed`, and destroy/drop must detach the blocked reader rather
/// than join it.
#[test]
fn destroy_detaches_reader_when_heir_holds_stdout() {
    let mut instance = L1Instance::spawn(&spec(&["--leave-heir-after-msgs=2"])).unwrap();

    wait_for_failed(&instance);
    let exit = wait_notification(&instance).expect("notification missing");
    // The worker exited cleanly — still `Failed` (live exit is a
    // violation) — with the real exit code reported.
    assert_eq!(exit.code, Some(0));

    let started = Instant::now();
    instance.destroy();
    drop(instance);
    assert!(
        started.elapsed() < NOTIFY_TIMEOUT,
        "teardown blocked on the EOF-starved reader"
    );
}

/// Shared-memory key retry (A5): a stale segment left behind at a
/// name the generator re-proposes must not fail the create — the
/// retry picks the next distinct key.
#[test]
fn shm_create_retries_past_stale_name() {
    use l1::shm::{Segment, unique_key};

    // Hold a real segment at a fixed key, then make the generator
    // collide with it on the first attempt.
    let stale = unique_key(0x5A);
    let _held = Segment::create_named(&stale, 1, 1).expect("stale segment create");
    let attempt = std::cell::Cell::new(0_u32);
    let generator = |slot: u32| {
        attempt.set(attempt.get() + 1);
        if attempt.get() == 1 {
            stale.clone()
        } else {
            unique_key(slot)
        }
    };
    let segment = Segment::create_keyed(7, 1, 1, &generator).expect("retry past the stale name");
    assert_ne!(segment.key(), stale);
    assert!(segment.verify_header(1, 1));
}

/// When every generated key collides the create gives up after the
/// bounded attempt count (`Err(())` → `HostError::Ipc` upstream).
#[test]
fn shm_create_gives_up_on_persistent_collision() {
    use l1::shm::{Segment, unique_key};

    let stale = unique_key(0x5B);
    let _held = Segment::create_named(&stale, 1, 1).expect("stale segment create");
    let generator = move |_slot: u32| stale.clone();
    assert!(Segment::create_keyed(7, 1, 1, &generator).is_err());
}

/// Layout consistency (Group C): the header written here decodes to
/// the same fields `wire::shm` defines — the single authoritative
/// layout — and `declared_len` equals `segment_len`.
#[test]
fn shm_header_roundtrip_matches_wire_layout() {
    use l1::shm::Segment;

    let in_depth = 2_u32;
    let out_depth = 4_u32;
    let segment = Segment::create(9, in_depth, out_depth).expect("segment create");
    assert!(segment.key().starts_with("/rvl1-shm-"));
    assert_eq!(
        segment.declared_len(),
        wire::shm::segment_len(in_depth, out_depth)
    );
    assert!(segment.verify_header(in_depth, out_depth));
    // Wrong depths must fail verification — the fields are real, not
    // a fixed stub.
    assert!(!segment.verify_header(in_depth + 1, out_depth));
    assert!(!segment.verify_header(in_depth, out_depth + 1));
}

/// `unique_key` must never repeat within the process — the counter
/// component discriminates even identical `slot` arguments.
#[test]
fn shm_unique_keys_never_repeat() {
    use l1::shm::unique_key;

    let mut keys = std::collections::HashSet::new();
    for _ in 0..64 {
        assert!(keys.insert(unique_key(3)));
    }
}

// ------------------------------------------------------------------
// Data plane (Issue #76 task 4): shm slots, doorbells, the RT legs.
// ------------------------------------------------------------------

/// A spawned instance driven to `Processing` — Initialize → Activate
/// (512-frame cap) → `StartProcessing`.
fn processing_instance(args: &[&str]) -> L1Instance {
    let mut instance = L1Instance::spawn(&spec(args)).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();
    instance
        .execute(wire::Command::Activate { config: config() })
        .unwrap();
    instance.execute(wire::Command::StartProcessing).unwrap();
    instance
}

/// One `process` call over mono planar views — the erased
/// `HostInput`/`HostOutput` surface the composite will use.
fn run_block(
    instance: &mut L1Instance,
    input: &[f32],
    output: &mut [f32],
) -> Result<ProcessResult, HostError> {
    let in_list = [input];
    let input_view = PlanarBufferView::new(&in_list).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut output[..]];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    instance.process(
        &inputs,
        &mut outputs,
        EventList::empty(u32::try_from(input.len()).unwrap()),
        None,
    )
}

/// One `RtLeg::process` call (the caller-thread half, no child) —
/// `mirror` is passed in so the leg is testable without an instance.
fn run_leg(
    leg: &mut l1::data::RtLeg,
    mirror: LifecycleState,
    input: &[f32],
    output: &mut [f32],
) -> Result<ProcessResult, HostError> {
    run_leg_full(
        leg,
        mirror,
        input,
        output,
        EventList::empty(u32::try_from(input.len()).unwrap()),
        None,
    )
}

/// `run_leg` with an explicit `EventList`/`TransportInfo` — the
/// encode paths in `stage_in` need real records under them.
fn run_leg_full(
    leg: &mut l1::data::RtLeg,
    mirror: LifecycleState,
    input: &[f32],
    output: &mut [f32],
    events: EventList<'_>,
    transport: Option<TransportInfo>,
) -> Result<ProcessResult, HostError> {
    let in_list = [input];
    let input_view = PlanarBufferView::new(&in_list).unwrap();
    let inputs = [HostInput(&input_view)];
    let mut out_list = [&mut output[..]];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    leg.process(mirror, &inputs, &mut outputs, events, transport)
}

/// Bit-exact sample comparison — the shm path round-trips `f32`
/// bit patterns through `to_bits`/`from_bits` and the worker gain
/// (×0.5) is exactly representable, so defined values compare equal
/// bit-for-bit (no epsilon needed or wanted).
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

/// Whether `sample` is a defined data-plane value: silence or the
/// gained (`×0.5`) copy of an input that could have completed by the
/// current call — never a torn mix, never garbage.
fn is_defined_sample(sample: f32, inputs: &[f32]) -> bool {
    sample_eq(sample, 0.0) || inputs.iter().any(|&v| sample_eq(sample, 0.5 * v))
}

/// Process roundtrip (task 4): the child applies `WORKER_GAIN`
/// (×0.5) over shared memory — when a doorbell completes inside the
/// lookahead window the output is exactly `0.5 · in[n−2]` (the
/// defined 2-block delay); when it has not arrived yet the leg
/// passes through `last_output`/silence. Every output sample is a
/// defined value — never a torn mix — and every call is
/// `Ok(CONTINUE)`.
#[test]
fn process_roundtrip_gain_and_lookahead_delay() {
    let mut instance = processing_instance(&[]);
    let frames = 512;
    let mut inputs: Vec<f32> = Vec::new();
    let mut matched = 0_u32;
    for call in 0..32_u32 {
        let value = 0.2 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
        let input = vec![value; frames];
        let mut output = vec![0.0_f32; frames];
        // A real-time-scale budget between calls — a 512-frame block
        // at 48 kHz is ~10.7 ms; 5 ms keeps the worker comfortably
        // ahead of the 2-block lookahead without racing it.
        thread::sleep(Duration::from_millis(5));
        let result = run_block(&mut instance, &input, &mut output);
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        if call >= 2 {
            let expected = 0.5 * inputs[call as usize - 2];
            if output.iter().all(|&s| sample_eq(s, expected)) {
                // The `call−2` block landed through the doorbell —
                // the defined lookahead delay, exactly.
                matched += 1;
            } else {
                // Underrun/pass-through — still only defined values:
                // silence or the gained copy of an older input.
                assert!(
                    output.iter().all(|&s| is_defined_sample(s, &inputs)),
                    "call {call}: output carried an undefined sample {}",
                    output[0]
                );
            }
        } else {
            // `seq - lookahead` underflow — defined silence.
            assert!(output.iter().all(|&s| sample_eq(s, 0.0)));
        }
        inputs.push(value);
    }
    // The pipeline must actually deliver: on a paced caller nearly
    // every doorbell completes inside its lookahead window; 16 of 30
    // leaves generous headroom for loaded CI hosts.
    assert!(
        matched >= 16,
        "the lookahead pipeline never filled — {matched} matched calls"
    );
    instance.destroy();
}

/// `process` outside `Processing` is the same `Lifecycle` rejection
/// L0 returns — before any buffer is touched.
#[test]
fn process_requires_processing_state() {
    let mut instance = L1Instance::spawn(&spec(&[])).unwrap();
    instance.execute(wire::Command::Initialize).unwrap();
    instance
        .execute(wire::Command::Activate { config: config() })
        .unwrap();
    let input = [0.5_f32; 512];
    let mut output = [1.0_f32; 512];
    assert!(matches!(
        run_block(&mut instance, &input, &mut output),
        Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
            from: LifecycleState::Activated,
            event: LifecycleEvent::Process,
        }))
    ));
    // The rejection precedes any copy — the output is untouched.
    assert!(output.iter().all(|&s| sample_eq(s, 1.0)));
    instance.destroy();
}

/// A block the wire cannot express (`frames >` activated
/// `max_frames`) is `Err(Ipc)` — the defined rejection, not a torn
/// slot write.
#[test]
fn process_over_capacity_block_is_ipc_error() {
    let mut instance = processing_instance(&[]);
    let input = vec![1.0_f32; 1024];
    let mut output = vec![0.0_f32; 1024];
    assert_eq!(
        run_block(&mut instance, &input, &mut output),
        Err(HostError::Ipc)
    );
    instance.destroy();
}

/// Healthy-underrun / `last_output` pass-through: rapid back-to-back
/// calls outrun the child's pipe turnaround, so the leg emits only
/// defined values (silence or the gained block) and never blocks;
/// once the caller paces at real-time scale the pipeline fills and
/// `0.5` lands exactly.
#[test]
fn process_underrun_passes_through_defined_output() {
    let mut instance = processing_instance(&[]);
    let input = [1.0_f32; 512];
    let mut output = vec![-1.0_f32; 512];
    // Back-to-back — most calls underrun: outputs are silence or the
    // last completed block, never garbage and never a hang.
    for _ in 0..16 {
        assert_eq!(
            run_block(&mut instance, &input, &mut output),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(
            output
                .iter()
                .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)),
            "underrun emitted an undefined sample"
        );
    }
    // Real-time pacing fills the pipeline — a completed block must
    // land, and `last_output` keeps it alive through later gaps.
    thread::sleep(Duration::from_millis(20));
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(5));
        run_block(&mut instance, &input, &mut output).unwrap();
    }
    assert!(
        output.iter().all(|&s| sample_eq(s, 0.5)),
        "the filled pipeline never delivered the gained block"
    );
    instance.destroy();
}

/// Crash while processing: the dead child answers nothing more —
/// the leg passes through (never an RT error, never a hang) until
/// convergence lands, then `Failed` yields defined silence. Every
/// call is `Ok(CONTINUE)`.
#[test]
fn process_after_crash_passes_through_then_silences() {
    let mut instance = processing_instance(&[]);
    let input = [1.0_f32; 512];
    let mut output = [0.0_f32; 512];
    // Get at least one doorbell genuinely in flight, then kill.
    run_block(&mut instance, &input, &mut output).unwrap();
    instance.kill();
    // Before `Failed` lands the leg still answers immediately.
    assert_eq!(
        run_block(&mut instance, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    wait_for_failed(&instance);
    output.fill(-1.0);
    assert_eq!(
        run_block(&mut instance, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(
        output.iter().all(|&s| sample_eq(s, 0.0)),
        "a Failed instance must output silence"
    );
    instance.destroy();
}

/// Wedged data plane: `--hang-after-msgs=5` lets the lifecycle
/// handshake through (`Scan`, `CreateInstance`, `Initialize`,
/// `Activate`, `StartProcessing`) but never answers a doorbell — the
/// RT leg must pass through `Ok(CONTINUE)` forever without blocking
/// (never a wall-clock wait on the child), and teardown still honors
/// `Shutdown`.
#[test]
fn process_hung_child_passes_through_without_blocking() {
    let mut instance = processing_instance(&["--hang-after-msgs=5"]);
    let input = [1.0_f32; 512];
    let mut output = [7.0_f32; 512];
    let started = Instant::now();
    for _ in 0..32 {
        assert_eq!(
            run_block(&mut instance, &input, &mut output),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(
            output.iter().all(|&s| sample_eq(s, 0.0)),
            "a child that never completes a block must output silence"
        );
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the RT leg blocked on the wedged child"
    );
    instance.destroy();
}

// ------------------------------------------------------------------
// Issue #117 bundled-child coverage at the `pub(crate)` layer: N
// facades on one `L1Child` — the `create_instance` + `from_parts`
// pair `PluginHost`'s `Bundled` route drives through its pool (the
// composite-level rows live in `composite_e2e` below).
// ------------------------------------------------------------------

/// Two facades on one `L1Child`, minted the pooled route's way.
fn bundled_pair(child: &Arc<L1Child>, spawn_spec: &SpawnSpec) -> (L1Instance, L1Instance) {
    let depth = spawn_spec.instance_depth().unwrap();
    let a = L1Instance::from_parts(
        Arc::clone(child),
        child.create_instance(spawn_spec, depth).unwrap(),
    );
    let b = L1Instance::from_parts(
        Arc::clone(child),
        child.create_instance(spawn_spec, depth).unwrap(),
    );
    (a, b)
}

/// The core bundle contract (issue-117 bundle-e2e rules): one child
/// process hosts two instances — distinct wire handles
/// (lowest-free mint), independent lifecycle machines (the mirror
/// *and* the child's own legality agree per instance), independent
/// state blobs, and independent data planes: interleaved doorbells
/// never cross — every output is silence or `0.5 ×` *that*
/// instance's own input history, so a cross-talked block would
/// surface as an undefined sample on the receiving side. Killing
/// the shared child fans `Failed` out to both facades, each
/// notification carrying the same `ChildExit`.
#[test]
fn bundled_child_hosts_independent_instances() {
    let spawn_spec = spec(&[]);
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let (mut a, mut b) = bundled_pair(&child, &spawn_spec);
    // One child, two wire handles — the lowest-free mint.
    assert_eq!(a.test_wire_instance(), 0);
    assert_eq!(b.test_wire_instance(), 1);
    assert_eq!(child.live_count(), 2);
    assert!(child.pool_eligible());
    assert_eq!(a.lifecycle_state(), LifecycleState::Created);
    assert_eq!(b.lifecycle_state(), LifecycleState::Created);

    // Lifecycle independence: `a` walks to `Processing` while `b`
    // stays `Created` — and the child's *own* legality agrees (an
    // `Activate` stamped for `b` is the per-instance
    // `Err(Lifecycle)`, not a child-scope violation).
    a.execute(wire::Command::Initialize).unwrap();
    a.execute(wire::Command::Activate { config: config() })
        .unwrap();
    a.execute(wire::Command::StartProcessing).unwrap();
    assert_eq!(b.lifecycle_state(), LifecycleState::Created);
    assert!(matches!(
        b.execute(wire::Command::Activate { config: config() }),
        Err(HostError::Lifecycle(_))
    ));

    // Per-instance state blobs — `a`'s `RestoreState` does not
    // touch `b`'s blob.
    a.execute(wire::Command::RestoreState { bytes: vec![0xAA] })
        .unwrap();
    assert_eq!(
        b.execute(wire::Command::SaveState),
        Ok(wire::Reply::State {
            bytes: vec![0x11, 0x22, 0x33, 0x44]
        })
    );
    b.execute(wire::Command::Initialize).unwrap();
    b.execute(wire::Command::Activate { config: config() })
        .unwrap();
    b.execute(wire::Command::StartProcessing).unwrap();

    // Interleaved processing on disjoint input ranges: the defined
    // set for each side is `{0} ∪ {0.5 × own history}` — any reply
    // or shm segment crossing instances lands outside it.
    let frames = 512;
    let mut a_inputs: Vec<f32> = Vec::new();
    let mut b_inputs: Vec<f32> = Vec::new();
    let mut delivered_a = false;
    let mut delivered_b = false;
    for call in 0..24_u32 {
        let va = 0.2 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
        let vb = 5.0 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
        let input_a = vec![va; frames];
        let input_b = vec![vb; frames];
        let mut out_a = vec![0.0_f32; frames];
        let mut out_b = vec![0.0_f32; frames];
        // A real-time-scale pace so most doorbells complete inside
        // the lookahead window on the shared serve loop.
        thread::sleep(Duration::from_millis(5));
        assert_eq!(
            run_block(&mut a, &input_a, &mut out_a),
            Ok(ProcessResult::CONTINUE)
        );
        assert_eq!(
            run_block(&mut b, &input_b, &mut out_b),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(
            out_a.iter().all(|&s| is_defined_sample(s, &a_inputs)),
            "call {call}: a carried a foreign/undefined sample {}",
            out_a[0]
        );
        assert!(
            out_b.iter().all(|&s| is_defined_sample(s, &b_inputs)),
            "call {call}: b carried a foreign/undefined sample {}",
            out_b[0]
        );
        delivered_a |= call >= 2
            && out_a
                .iter()
                .all(|&s| sample_eq(s, 0.5 * a_inputs[call as usize - 2]));
        delivered_b |= call >= 2
            && out_b
                .iter()
                .all(|&s| sample_eq(s, 0.5 * b_inputs[call as usize - 2]));
        a_inputs.push(va);
        b_inputs.push(vb);
    }
    assert!(delivered_a, "a's pipeline never delivered a block");
    assert!(delivered_b, "b's pipeline never delivered a block");

    // The kill is child-scoped: one `kill()` fans `Failed` out to
    // every live facade on the shared child — each instance's own
    // notification channel carries the same `ChildExit`.
    a.kill();
    wait_for_failed(&a);
    wait_for_failed(&b);
    let exit_a = wait_notification(&a).expect("a's notification missing");
    let exit_b = wait_notification(&b).expect("b's notification missing");
    assert_eq!(exit_a, exit_b);
    assert!(exit_a.code.is_some() || exit_a.signal.is_some());
    assert!(a.try_recv_notification().is_none());
    assert!(b.try_recv_notification().is_none());
    // Converged entries stay registered until `Destroy` reclaims
    // the runner slots.
    assert_eq!(child.live_count(), 2);
    a.destroy();
    b.destroy();
    assert_eq!(child.live_count(), 0);
    assert!(child.is_terminal());
}

/// The instance-scope convergence at wire fidelity: an instance
/// failed *alone* (the demux's attributable violation — stamped
/// through the `test_converge_instance` seam because the worker
/// has no instance-scope fault vocabulary) stays registered on the
/// live child; `Destroy` on it still sends the real
/// `DestroyInstance` — the runner-slot reclaim — while the sibling
/// keeps serving, and the freed wire handle becomes the next
/// lowest-free mint.
#[test]
fn bundled_failed_instance_destroy_recovers_handle() {
    let spawn_spec = spec(&[]);
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let (mut a, mut b) = bundled_pair(&child, &spawn_spec);

    // Fail `a` alone — the attributable instance-scope violation
    // (`ChildExit{None, None}`: the child is alive, there is no
    // exit status to report).
    child.test_converge_instance(0);
    assert_eq!(a.lifecycle_state(), LifecycleState::Failed);
    assert_eq!(
        a.try_recv_notification(),
        Some(ChildExit {
            code: None,
            signal: None
        })
    );
    assert!(a.try_recv_notification().is_none());
    assert_eq!(b.lifecycle_state(), LifecycleState::Created);
    assert!(!child.is_terminal());
    assert_eq!(child.live_count(), 2);

    // The sibling is untouched — the child still serves its
    // commands (per-instance granularity, not child-scope).
    b.execute(wire::Command::Initialize).unwrap();
    assert_eq!(b.lifecycle_state(), LifecycleState::Initialized);

    // `Destroy` on the failed instance still goes on the wire —
    // the runner's registry slot is really reclaimed.
    a.destroy();
    assert_eq!(a.lifecycle_state(), LifecycleState::Destroyed);
    assert_eq!(child.live_count(), 1);
    assert!(!child.is_terminal());

    // The reclaimed handle is the lowest-free mint: the next
    // `create_instance` re-uses wire handle 0 — and the runner
    // accepting it proves the previous slot left its registry.
    let mut c = L1Instance::from_parts(
        Arc::clone(&child),
        child
            .create_instance(&spawn_spec, spawn_spec.instance_depth().unwrap())
            .unwrap(),
    );
    assert_eq!(c.test_wire_instance(), 0);
    assert_eq!(child.live_count(), 2);
    c.execute(wire::Command::Initialize).unwrap();

    b.destroy();
    c.destroy();
}

/// Shutdown-on-empty at the `pub(crate)` layer: a mid-bundle
/// `Destroy` leaves the shared child serving the surviving
/// sibling, and the last *facade* reference's `destroy` runs the
/// bounded `Shutdown`+reap teardown itself (the
/// `Arc::strong_count == 1` rule — the composite's pool eviction
/// reads the same `live_count` at route level).
#[test]
fn bundled_last_facade_teardown_keeps_siblings_alive() {
    let spawn_spec = spec(&[]);
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let (mut a, mut b) = bundled_pair(&child, &spawn_spec);
    a.execute(wire::Command::Initialize).unwrap();
    b.execute(wire::Command::Initialize).unwrap();
    let weak = Arc::downgrade(&child);

    // `a`'s `Destroy` reclaims its slot — `b` (plus this test's own
    // `child` reference) keeps the child alive: no teardown.
    a.destroy();
    assert_eq!(child.live_count(), 1);
    assert!(!child.is_terminal());
    assert_eq!(b.lifecycle_state(), LifecycleState::Initialized);
    // The surviving sibling still serves — `a`'s exit never
    // reached a `Shutdown`.
    assert_eq!(
        b.execute(wire::Command::SaveState),
        Ok(wire::Reply::State {
            bytes: vec![0x11, 0x22, 0x33, 0x44]
        })
    );

    // Shed every reference except `b`'s own facade Arc — `b`'s
    // `destroy` then sees `strong_count == 1` and runs the bounded
    // `Shutdown`+grace+reap teardown inline.
    drop(a);
    drop(child);
    assert!(weak.upgrade().is_some());
    b.destroy();
    assert_eq!(b.lifecycle_state(), LifecycleState::Destroyed);
    // The teardown stamped `terminal` — `b.child()` keeps the Arc
    // alive long enough to observe it (a clean teardown never
    // notifies).
    assert!(b.child().is_terminal());
    assert!(b.try_recv_notification().is_none());
    drop(b);
    assert!(weak.upgrade().is_none());
}

/// Mid-`DestroyInstance` death at the `pub(crate)` layer: the
/// child dies on receipt of `a`'s `DestroyInstance` (message 4 —
/// after `Scan` + two `CreateInstance`s) — `a.destroy()` still
/// completes (`Destroyed`, entry removed) and its minted
/// notification still lands on the facade's own queue; `b`
/// converges `Failed` by the same child-scope fan-out.
#[test]
fn bundled_mid_destroy_death_keeps_notification() {
    let spawn_spec = spec(&["--crash-after-msgs=4"]);
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let (mut a, mut b) = bundled_pair(&child, &spawn_spec);

    // `DestroyInstance` is the crashing message — the request
    // fails `Err(Ipc)` but `destroy` proceeds to removal.
    a.destroy();
    assert_eq!(a.lifecycle_state(), LifecycleState::Destroyed);
    // The minted notification survives entry removal — the
    // `Pending` flush / already-`Sent` race resolves either way,
    // but exactly one notification must exist.
    assert!(a.try_recv_notification().is_some());
    assert!(a.try_recv_notification().is_none());

    // `b` is the child-scope sibling: `Failed`, notified once.
    wait_for_failed(&b);
    let exit_b = wait_notification(&b).expect("b's notification missing");
    assert!(exit_b.code.is_some() || exit_b.signal.is_some());
    assert!(b.try_recv_notification().is_none());
    b.destroy();
    assert!(child.is_terminal());
}

/// `Err(InstanceLimit)` at the real-child layer: `create_instance`
/// mints up to `WIRE_MAX_INSTANCES` wire handles on one child,
/// then answers the defined capacity rejection — the same fact the
/// pooled route distinguishes from `Ipc` when it decides to spawn
/// the next child (host-internal: `Instantiate` can never observe
/// it — a full child makes the pool spawn first).
#[test]
fn bundled_create_past_capacity_is_instance_limit() {
    let spawn_spec = spec(&[]);
    let depth = spawn_spec.instance_depth().unwrap();
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let mut parts = Vec::new();
    for _ in 0..wire::WIRE_MAX_INSTANCES {
        parts.push(child.create_instance(&spawn_spec, depth).unwrap());
    }
    assert_eq!(child.live_count(), wire::WIRE_MAX_INSTANCES as usize);
    assert!(!child.pool_eligible());
    assert!(!child.is_terminal());
    assert_eq!(
        child.create_instance(&spawn_spec, depth).map(|_| ()),
        Err(HostError::InstanceLimit)
    );
    // The child stayed healthy — the rejection minted nothing.
    assert_eq!(child.live_count(), wire::WIRE_MAX_INSTANCES as usize);
    drop(parts);
    child.teardown();
}

// ------------------------------------------------------------------
// RT contract tests (the `rt_l1_*` lane `mise run test-rt` selects).
// ------------------------------------------------------------------

/// `RtLeg::process` — the whole caller-thread path (lifecycle gate →
/// in-staging write → `try_push` → bounded result drain → out-copy /
/// pass-through) — must run allocation-free, *including* the legs
/// the earlier version of this test never reached: `stage_in`'s
/// event/transport wire encodes (the `scratch` Vec paths — sized so
/// no encode can outgrow the preallocated capacity), the `held`
/// stash, `copy_out` and the `last_output` pass-through history
/// update. Fixture, staging, the channels, the fabricated slot
/// buffer and warm-up all sit outside the `assert_no_alloc` window;
/// inside it, results are fabricated via `read_out_slot` (atomic
/// stores only) and pushed through the real result channel (a
/// bounded ring `try_push`) so `copy_out`/`held`/`pass_through` run
/// for real.
#[test]
fn rt_l1_process_path_no_alloc() {
    use l1::data::{BLOCK_SEQ_FLAG, RtLeg, Staging, Submit, read_out_slot};
    use l1::shm::{put_u32, put_u64};

    const FRAMES: usize = 512;
    let (submit_tx, _submit_rx) = ruviform_rt::channel::<Submit>(3);
    let (mut result_tx, result_rx) = ruviform_rt::channel::<u64>(3);
    let mut leg = RtLeg::new(3, 2, submit_tx, result_rx);
    let staging = Arc::new(Staging::new(3, u32::try_from(FRAMES).unwrap(), 0));
    leg.arm(Arc::clone(&staging));

    // Every call carries an event + a transport record so the
    // `stage_in` encode loops run inside the window.
    let events = [Event::new(
        64,
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(0),
            value: ParamValue::new(0.75).unwrap(),
        }),
    )];
    let event_list = EventList::new(&events, u32::try_from(FRAMES).unwrap()).unwrap();
    let transport = TransportInfo::new(TransportStatus::Playing, MediaTime::ZERO);

    let input = [1.0_f32; FRAMES];
    let mut output = [0.0_f32; FRAMES];
    // The out-slot the reader would publish — allocated here once,
    // then re-stamped per seq (the `read_out_slot` fabrications are
    // atomic stores on the preallocated stage; the result channel is
    // a bounded ring). `read_out_slot` returns `false` only on a
    // protocol violation — impossible for this well-formed slot.
    let mut slot = vec![0_u8; wire::shm::SLOT_BYTES];
    put_u32(
        &mut slot,
        wire::shm::SLOT_OFF_FRAMES,
        u32::try_from(FRAMES).unwrap(),
    );
    put_u32(&mut slot, wire::shm::SLOT_OFF_CHANNELS, 1);
    for frame in 0..FRAMES {
        let base = wire::shm::SAMPLES_OFF + frame * 4;
        slot[base..base + 4].copy_from_slice(&0.5_f32.to_le_bytes());
    }
    // Fabricates + queues block `n`'s result exactly like the
    // reader's `BlockDone` leg (allocation-free by construction).
    let mut feed_result = |block: u64| {
        let wire_seq = block | BLOCK_SEQ_FLAG;
        put_u64(&mut slot, wire::shm::SLOT_OFF_SEQ, wire_seq);
        let out_stage = usize::try_from(wire_seq % 3).unwrap();
        assert!(read_out_slot(
            &slot,
            wire_seq,
            &staging.outs[out_stage],
            FRAMES,
            ProcessResult::CONTINUE,
        ));
        result_tx.try_push(wire_seq).unwrap();
    };

    // Warm-up outside the window: first-touch staging pages, channel
    // state, per-call view construction and real matched results
    // (establishing `last_output` history) all belong to fixture
    // setup.
    for call in 0..4_u64 {
        if call >= 2 {
            feed_result(call - 2);
        }
        run_leg_full(
            &mut leg,
            LifecycleState::Processing,
            &input,
            &mut output,
            event_list,
            Some(transport),
        )
        .unwrap();
    }

    // `block` is the leg's monotone counter — it has ticked once per
    // call above. Inside the window: `n % 3 == 0` feeds a *future*
    // seq (stashed in `held`, resolved next call) while `expected`
    // stays absent → pass-through; `n % 3 == 1` resolves `expected`
    // out of `held`; `n % 3 == 2` feeds `expected` directly →
    // channel-matched `copy_out`. Every path runs under the
    // forbid-allocation window.
    let mut block: u64 = 4;
    assert_no_alloc::assert_no_alloc(|| {
        for n in 0..48_u64 {
            let expected = block - 2;
            match n % 3 {
                0 => feed_result(expected + 1),
                2 => feed_result(expected),
                _ => {}
            }
            let result = run_leg_full(
                &mut leg,
                LifecycleState::Processing,
                &input,
                &mut output,
                event_list,
                Some(transport),
            )
            .unwrap();
            assert_eq!(result, ProcessResult::CONTINUE);
            assert!(
                output
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5))
            );
            block += 1;
        }
    });
}

/// The result leg is `try_push`/`try_pop` end to end: a reader-side
/// `try_push` of the done seq is consumed by the leg's bounded
/// `try_pop` drain — matched results copy out, a failed result
/// (`mark_failed`) and stale seqs fall back to the defined
/// pass-through, and the submit leg's doorbell seqs carry
/// `BLOCK_SEQ_FLAG` (the command-space partition).
#[test]
fn rt_l1_result_leg_try_ops() {
    use l1::data::{BLOCK_SEQ_FLAG, RtLeg, Staging, Submit, mark_failed, read_out_slot};
    use l1::shm::{put_u32, put_u64};

    const FRAMES: usize = 512;
    let staging = Arc::new(Staging::new(3, u32::try_from(FRAMES).unwrap(), 0));
    let (submit_tx, mut submit_rx) = ruviform_rt::channel::<Submit>(3);
    let (mut result_tx, result_rx) = ruviform_rt::channel::<u64>(3);
    let mut leg = RtLeg::new(3, 2, submit_tx, result_rx);
    leg.arm(Arc::clone(&staging));

    let input = [1.0_f32; FRAMES];
    let mut output = [0.0_f32; FRAMES];
    // Calls 0/1: `seq - lookahead` underflows — silence, and the
    // doorbell seqs on the submit channel carry the partition flag
    // plus the armed ring's generation.
    for n in 0..2_u64 {
        assert_eq!(
            run_leg(&mut leg, LifecycleState::Processing, &input, &mut output),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(output.iter().all(|&s| sample_eq(s, 0.0)));
        assert_eq!(
            submit_rx.try_pop(),
            Ok(Submit {
                seq: BLOCK_SEQ_FLAG | n,
                generation: 0
            })
        );
    }

    // Build the out-slot the reader would publish for seq 0 — the
    // authoritative wire layout via `read_out_slot`, no shortcuts.
    // NB: the stage index is `seq % depth` on the *flagged* wire seq
    // (identically to the shm slot), which for `FLAG|0` is `2`.
    let out_stage = usize::try_from(BLOCK_SEQ_FLAG % 3).unwrap();
    let mut slot = vec![0_u8; wire::shm::SLOT_BYTES];
    put_u64(&mut slot, wire::shm::SLOT_OFF_SEQ, BLOCK_SEQ_FLAG);
    put_u32(
        &mut slot,
        wire::shm::SLOT_OFF_FRAMES,
        u32::try_from(FRAMES).unwrap(),
    );
    put_u32(&mut slot, wire::shm::SLOT_OFF_CHANNELS, 1);
    for frame in 0..FRAMES {
        let base = wire::shm::SAMPLES_OFF + frame * 4;
        slot[base..base + 4].copy_from_slice(&0.5_f32.to_le_bytes());
    }
    assert!(read_out_slot(
        &slot,
        BLOCK_SEQ_FLAG,
        &staging.outs[out_stage],
        FRAMES,
        ProcessResult::CONTINUE,
    ));
    result_tx.try_push(BLOCK_SEQ_FLAG).unwrap();

    // Call 2 completes block 0 — matched result → the child's 0.5.
    assert_eq!(
        run_leg(&mut leg, LifecycleState::Processing, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(output.iter().all(|&s| sample_eq(s, 0.5)));

    // Call 3 expects seq 1 — never answered → `last_output` (0.5).
    assert_eq!(
        run_leg(&mut leg, LifecycleState::Processing, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(output.iter().all(|&s| sample_eq(s, 0.5)));

    // The child's flagged `Reply` rejection lands as `mark_failed` —
    // call 4 expects seq 2 → failed result → pass-through, never the
    // (zero) staged slot.
    result_tx.try_push(mark_failed(BLOCK_SEQ_FLAG | 2)).unwrap();
    assert_eq!(
        run_leg(&mut leg, LifecycleState::Processing, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(output.iter().all(|&s| sample_eq(s, 0.5)));

    // A late seq-1 result is stale by call 5 — drained and dropped
    // without disturbing the `seq 3` expectation.
    result_tx.try_push(BLOCK_SEQ_FLAG | 1).unwrap();
    output.fill(-1.0);
    assert_eq!(
        run_leg(&mut leg, LifecycleState::Processing, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(output.iter().all(|&s| sample_eq(s, 0.5)));

    // Failed-mirror leg: defined silence, `Ok(CONTINUE)`.
    output.fill(-1.0);
    assert_eq!(
        run_leg(&mut leg, LifecycleState::Failed, &input, &mut output),
        Ok(ProcessResult::CONTINUE)
    );
    assert!(output.iter().all(|&s| sample_eq(s, 0.0)));
}

/// The bundled RT contract: two `Processing` facades on *one* live
/// `L1Child` — each `process` call exercises its own staging,
/// submit/result rings and `rt_state` snapshot, and the caller-thread
/// path stays allocation-free under `assert_no_alloc` exactly like
/// the singleton leg (per-instance state must not share a hot-path
/// allocation). Child/facade setup and the warm-up blocks sit
/// outside the window; inside it, real doorbells are in flight on
/// the shared child while both legs run — defined outputs only.
#[test]
fn rt_l1_bundled_process_no_alloc() {
    let spawn_spec = spec(&[]);
    let child = L1Child::spawn_scanned(&spawn_spec).unwrap();
    let (mut a, mut b) = bundled_pair(&child, &spawn_spec);
    for instance in [&mut a, &mut b] {
        instance.execute(wire::Command::Initialize).unwrap();
        instance
            .execute(wire::Command::Activate { config: config() })
            .unwrap();
        instance.execute(wire::Command::StartProcessing).unwrap();
    }

    let input = [0.5_f32; 512];
    let mut out_a = [0.0_f32; 512];
    let mut out_b = [0.0_f32; 512];
    // Warm-up outside the window: first-touch staging pages, channel
    // state and the driver's `AddInstance` attach belong to setup.
    for _ in 0..4 {
        run_block(&mut a, &input, &mut out_a).unwrap();
        run_block(&mut b, &input, &mut out_b).unwrap();
    }
    assert_no_alloc::assert_no_alloc(|| {
        for _ in 0..24 {
            assert_eq!(
                run_block(&mut a, &input, &mut out_a),
                Ok(ProcessResult::CONTINUE)
            );
            assert_eq!(
                run_block(&mut b, &input, &mut out_b),
                Ok(ProcessResult::CONTINUE)
            );
            // Defined values only — silence or the child's `0.5 ×`
            // (underrun pass-through included), never a torn mix.
            assert!(
                out_a
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.25))
            );
            assert!(
                out_b
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.25))
            );
        }
    });
    // Both legs stayed `Processing` — no cross-talk, no convergence.
    assert_eq!(a.lifecycle_state(), LifecycleState::Processing);
    assert_eq!(b.lifecycle_state(), LifecycleState::Processing);
    a.destroy();
    b.destroy();
}

/// `lookahead_blocks == 0` would degenerate the data plane to
/// permanent silence — `expected` would be the very seq the same
/// `process` call just submitted, so no result could ever precede
/// it. `spawn` rejects the spec `Err(Ipc)` like any other unusable
/// configuration.
#[test]
fn lookahead_zero_is_rejected() {
    let mut spec = spec(&[]);
    spec.lookahead_blocks = 0;
    assert_eq!(L1Instance::spawn(&spec).map(|_| ()), Err(HostError::Ipc));
}

/// `D = lookahead_blocks + 1` is declared on `CreateInstance`'s
/// `in_depth`/`out_depth`, which the wire codec caps at
/// `WIRE_MAX_ELEMENTS` — an over-cap lookahead is the same defined
/// `Err(Ipc)` rejection, before any segment or child exists (so the
/// `u32::MAX` case also proves the `+ 1` cannot overflow).
#[test]
fn lookahead_over_wire_depth_cap_is_rejected() {
    let mut spec = spec(&[]);
    spec.lookahead_blocks = wire::WIRE_MAX_ELEMENTS;
    assert_eq!(L1Instance::spawn(&spec).map(|_| ()), Err(HostError::Ipc));
    spec.lookahead_blocks = u32::MAX;
    assert_eq!(L1Instance::spawn(&spec).map(|_| ()), Err(HostError::Ipc));
}

/// Submits carry the armed ring's generation — the driver-side gate
/// (`armed_for`) then drops a submit queued under a superseded ring
/// instead of serializing the new ring's zeroed stage under a real
/// seq (the re-activation stale-submit case). A dropped seq still
/// completes: the driver synthesizes a `mark_failed` result on the
/// shared result producer so the leg's alternation gate frees the
/// slot and resolves the block as the defined pass-through.
#[test]
fn submit_carries_staging_generation_and_stale_is_dropped() {
    use l1::data::{BLOCK_SEQ_FLAG, RtLeg, Staging, Submit, armed_for};

    let (submit_tx, mut submit_rx) = ruviform_rt::channel::<Submit>(3);
    let (_result_tx, result_rx) = ruviform_rt::channel::<u64>(3);
    let mut leg = RtLeg::new(3, 2, submit_tx, result_rx);
    leg.arm(Arc::new(Staging::new(3, 512, 7)));

    let input = [1.0_f32; 512];
    let mut output = [0.0_f32; 512];
    run_leg(&mut leg, LifecycleState::Processing, &input, &mut output).unwrap();
    assert_eq!(
        submit_rx.try_pop(),
        Ok(Submit {
            seq: BLOCK_SEQ_FLAG,
            generation: 7
        })
    );

    // Re-activation installs a new ring (generation 8) — submits
    // queued under generation 7 are stale from here on.
    let new_ring = Arc::new(Staging::new(3, 512, 8));
    leg.arm(Arc::clone(&new_ring));
    run_leg(&mut leg, LifecycleState::Processing, &input, &mut output).unwrap();
    assert_eq!(
        submit_rx.try_pop(),
        Ok(Submit {
            seq: BLOCK_SEQ_FLAG | 1,
            generation: 8
        })
    );

    // The driver's gate: a stale-generation submit is dropped; the
    // current generation still doorbells; no armed ring drops too.
    let armed = Some(Arc::clone(&new_ring));
    assert!(
        armed_for(
            armed.as_ref(),
            Submit {
                seq: BLOCK_SEQ_FLAG,
                generation: 7
            }
        )
        .is_none(),
        "a pre-swap submit must never be doorbell'd"
    );
    let kept = armed_for(
        armed.as_ref(),
        Submit {
            seq: BLOCK_SEQ_FLAG | 1,
            generation: 8,
        },
    );
    assert!(kept.is_some_and(|ring| Arc::ptr_eq(&ring, &new_ring)));
    assert!(
        armed_for(
            None,
            Submit {
                seq: BLOCK_SEQ_FLAG | 1,
                generation: 8
            }
        )
        .is_none()
    );
}

/// Rapid re-activation while doorbells may still be queued: submits
/// staged under the superseded ring are dropped by the generation
/// gate — never serialized from the new ring's zeroed stage and
/// doorbell'd under a real seq. The worker answers a zero-dim
/// (never-staged) block with the `ZERO_DIM_RESULT` marker
/// (`ProcessResult{Sleep, 0xDEAD}`), so any stale doorbell that did
/// go through would surface as that unmistakable result rather than
/// `CONTINUE`; stale seqs that resolve at all must resolve as
/// failed/pass-through.
#[test]
fn reactivation_cycle_never_publishes_stale_blocks() {
    /// The worker's `ZERO_DIM_RESULT` marker — kept local so the
    /// fixture constant stays in one place.
    const ZERO_DIM_RESULT: ProcessResult = ProcessResult::new(ProcessStatus::Sleep, 0xDEAD);

    let mut instance = processing_instance(&[]);
    let input = [1.0_f32; 512];
    let mut output = [0.0_f32; 512];
    for cycle in 0..32 {
        // Queue doorbells as close to the ring swap as the API
        // allows, then cycle the lifecycle through re-activation.
        for _ in 0..3 {
            run_block(&mut instance, &input, &mut output).unwrap();
        }
        instance.execute(wire::Command::StopProcessing).unwrap();
        instance.execute(wire::Command::Deactivate).unwrap();
        instance
            .execute(wire::Command::Activate { config: config() })
            .unwrap();
        instance.execute(wire::Command::StartProcessing).unwrap();
        for call in 0..6 {
            let result = run_block(&mut instance, &input, &mut output);
            assert_ne!(
                result,
                Ok(ZERO_DIM_RESULT),
                "cycle {cycle} call {call}: a zeroed block was doorbell'd and published"
            );
            assert_eq!(result, Ok(ProcessResult::CONTINUE));
            assert!(
                output
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)),
                "cycle {cycle} call {call}: output carried an undefined sample"
            );
        }
    }
    instance.destroy();
}

/// Hostile child dims (F1): an out-slot reporting `frames` over the
/// activated `frame_capacity` (or `channels` over the wire bound) is
/// a protocol violation — `read_out_slot` withholds it (returns
/// `false`, publishes nothing, and the reader survives without an
/// out-of-bounds index) instead of panicking or leaking cross-plane
/// corruption through the `Release` store.
#[test]
fn read_out_slot_rejects_dims_over_activated_contract() {
    use l1::data::{BLOCK_SEQ_FLAG, Staging, read_out_slot};
    use l1::shm::{put_u32, put_u64};

    const FRAMES: usize = 512;
    let out_stage = usize::try_from(BLOCK_SEQ_FLAG % 3).unwrap();
    let seq = BLOCK_SEQ_FLAG;
    let mut slot = vec![0_u8; wire::shm::SLOT_BYTES];
    put_u64(&mut slot, wire::shm::SLOT_OFF_SEQ, seq);

    // In-contract dims still publish (the activated bound is the
    // bound, not the wire max — `FRAMES` here is the capacity).
    let staging = Staging::new(3, u32::try_from(FRAMES).unwrap(), 0);
    put_u32(
        &mut slot,
        wire::shm::SLOT_OFF_FRAMES,
        u32::try_from(FRAMES).unwrap(),
    );
    put_u32(&mut slot, wire::shm::SLOT_OFF_CHANNELS, 1);
    assert!(read_out_slot(
        &slot,
        seq,
        &staging.outs[out_stage],
        FRAMES,
        ProcessResult::CONTINUE,
    ));
    assert_eq!(
        staging.outs[out_stage].seq.load(Ordering::Acquire),
        seq,
        "a published stage carries the seq tag"
    );

    // Every over-contract dim is withheld — `frames` past the
    // activated capacity (the pre-fix panic path) or `channels`
    // past `WIRE_MAX_CHANNELS` — and nothing is published (the seq
    // tag stays at its initial value, so the RT leg's tag check
    // fails closed).
    for (frames, channels) in [
        (8_192_u32, 1_u32),
        (u32::MAX, 1),
        (513, 1),
        (512, 17),
        (512, u32::MAX),
    ] {
        let staging = Staging::new(3, u32::try_from(FRAMES).unwrap(), 0);
        put_u32(&mut slot, wire::shm::SLOT_OFF_FRAMES, frames);
        put_u32(&mut slot, wire::shm::SLOT_OFF_CHANNELS, channels);
        assert!(
            !read_out_slot(
                &slot,
                seq,
                &staging.outs[out_stage],
                FRAMES,
                ProcessResult::CONTINUE,
            ),
            "frames={frames} channels={channels} must be withheld"
        );
        assert_eq!(
            staging.outs[out_stage].seq.load(Ordering::Acquire),
            0,
            "a rejected slot must not publish (frames={frames} channels={channels})"
        );
    }
}

/// Stale-plane hygiene (F2): an outputs-only block (or a channel
/// slice missing from the input bus) still claims `block.channels`
/// planes in `stage.channels` — `stage_in` must serialize zeros for
/// every claimed plane, never the previous occupant's samples.
#[test]
fn stage_in_zero_fills_planes_without_a_source() {
    use l1::data::{BLOCK_SEQ_FLAG, RtLeg, Staging, Submit, write_in_slot};
    use l1::shm::get_u32;

    const FRAMES: usize = 512;
    let (submit_tx, _submit_rx) = ruviform_rt::channel::<Submit>(3);
    let (mut result_tx, result_rx) = ruviform_rt::channel::<u64>(3);
    let mut leg = RtLeg::new(3, 2, submit_tx, result_rx);
    let staging = Arc::new(Staging::new(3, u32::try_from(FRAMES).unwrap(), 0));
    leg.arm(Arc::clone(&staging));

    // The in-slot index for wire seq `FLAG|n` is `(FLAG|n) % 3`, so
    // the outputs-only block at call 4 (`FLAG|4`, index 0) reuses
    // the slot block 1 (`FLAG|1`) staged its samples into.
    let reuse_seq = BLOCK_SEQ_FLAG | 4;
    let slot_index = usize::try_from(reuse_seq % 3).unwrap();
    let input = [1.0_f32; FRAMES];
    let mut output = [0.0_f32; FRAMES];

    // Blocks 0..2 stage real samples into the three in-slots.
    for _ in 0..3 {
        run_leg(&mut leg, LifecycleState::Processing, &input, &mut output).unwrap();
    }
    // Let block 1's completion land — the drained seq clears
    // `cleared[0]`, opening slot 0's alternation gate for block 4
    // (the staging tag check need not match; the seq alone does it).
    result_tx.try_push(BLOCK_SEQ_FLAG | 1).unwrap();
    run_leg(&mut leg, LifecycleState::Processing, &input, &mut output).unwrap();

    // Block 4 (wire seq `FLAG|4`, slot 0 again) with NO input bus —
    // an outputs-only block: `block.channels` still claims the
    // output's channel count, so `stage_in` must zero-fill the
    // plane rather than leave block 1's samples behind.
    let mut out_list = [&mut output[..]];
    let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
    let mut outputs = [HostOutput(&mut output_view)];
    leg.process(
        LifecycleState::Processing,
        &[],
        &mut outputs,
        EventList::empty(u32::try_from(FRAMES).unwrap()),
        None,
    )
    .unwrap();

    // Serialize the staged slot exactly as the driver would and
    // verify the claimed plane is all zeros.
    let mut slot = vec![0_u8; wire::shm::SLOT_BYTES];
    write_in_slot(&mut slot, reuse_seq, &staging.ins[slot_index], FRAMES);
    assert_eq!(
        get_u32(&slot, wire::shm::SLOT_OFF_FRAMES),
        u32::try_from(FRAMES).unwrap()
    );
    assert_eq!(get_u32(&slot, wire::shm::SLOT_OFF_CHANNELS), 1);
    for frame in 0..FRAMES {
        let base = wire::shm::SAMPLES_OFF + frame * 4;
        assert_eq!(
            &slot[base..base + 4],
            &0.0_f32.to_le_bytes(),
            "plane 0 frame {frame} leaked stale data"
        );
    }
}

/// A real flagged-`Reply` block rejection (F7): after
/// `--reject-blocks-after-msgs=2` the worker answers each `BlockIn`
/// past the first two with `Reply{seq, Err(PluginFailed)}` — the
/// defined rejection shape — which the reader lands as
/// `mark_failed(seq)`. Every expected seq past the flag resolves
/// `Ok(CONTINUE)` with pass-through output (`last_output` or
/// silence), never a successful publish of the rejected block and
/// never a hang.
#[test]
fn rejected_block_replies_resolve_pass_through() {
    // Seqs 0 and 1 process normally (gain ×0.5); seqs 2+ get the
    // flagged rejection.
    let mut instance = processing_instance(&["--reject-blocks-after-msgs=2"]);
    let mut inputs: Vec<f32> = Vec::new();
    let mut processed = 0_u32;
    for call in 0..12_u32 {
        // Distinct inputs per call so a rejected block's
        // pass-through can never alias its would-be-processed
        // value.
        let value = 0.2 + 0.1 * f32::from(u8::try_from(call).unwrap());
        let input = vec![value; 512];
        let mut output = vec![7.0_f32; 512];
        thread::sleep(Duration::from_millis(5));
        let result = run_block(&mut instance, &input, &mut output);
        assert_eq!(result, Ok(ProcessResult::CONTINUE));
        assert!(
            output.iter().all(|&s| is_defined_sample(s, &inputs)),
            "call {call}: output carried an undefined sample"
        );
        if (2..4).contains(&call) {
            // Calls expecting seqs 0/1 — the pipeline must have
            // actually delivered at least once before the rejection
            // window, or the test proves nothing.
            if output
                .iter()
                .all(|&s| sample_eq(s, 0.5 * inputs[call as usize - 2]))
            {
                processed += 1;
            }
        }
        if call >= 4 {
            // Calls expecting seqs 2+ — rejected: the output is a
            // defined pass-through, never `0.5 *` this block's
            // input.
            let processed_value = 0.5 * inputs[call as usize - 2];
            assert!(
                output.iter().any(|&s| !sample_eq(s, processed_value)),
                "call {call}: a rejected block published processed output"
            );
        }
        inputs.push(value);
    }
    assert!(
        processed >= 1,
        "the lookahead pipeline never delivered a processed block"
    );
    instance.destroy();
}

// ------------------------------------------------------------------
// Composite (`PluginHost`) public surface (Issue #76 task 5):
// unified handle space, registration-time L0/L1 routing and the
// public notification surface — exercised end-to-end over both the
// `l1-test-worker` fixture and the real `ruviform-plugin-l1-runner`
// bin from `ruviform-plugin-export`.
// ------------------------------------------------------------------

mod composite_e2e {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant};

    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{
        ErasedPlugin, Event, EventKind, EventList, GuiParentToken, HostError, HostInput,
        HostOutput, LifecycleError, LifecycleEvent, LifecycleState, ParamEvent, ParamId, ParamInfo,
        ParamValue, PluginDescriptor, ProcessConfig, ProcessResult, TransportInfo,
    };
    use ruviform_plugin_host::{
        HostCommand, HostNotification, HostReply, OutOfProcessIsolation, OutOfProcessSpec, Plugin,
        PluginFactory, PluginHandle, PluginHost,
    };

    use super::{NOTIFY_TIMEOUT, WORKER_DESCRIPTOR, config, sample_eq};

    /// The L0-side fixture: a gain plugin with an adjustable
    /// normalized-gain parameter and a state blob — distinct id and
    /// descriptor from `l1-test-worker`.
    const L0_GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.l0-gain",
        name: "L0 Test Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

    /// The descriptor `ruviform-plugin-l1-runner`'s built-in
    /// `GainPlugin` reports — the `OutOfProcessSpec` oracle must
    /// field-match it.
    const RUNNER_GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "dev.ruviform.mock.gain",
        name: "Gain",
        vendor: "Ruviform",
        version: "1.0.0",
        features: &["audio-effect"],
    };

    struct L0Gain {
        gain: f32,
    }

    impl ruviform_plugin_core::PluginProcessor for L0Gain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ruviform_plugin_core::ProcessContext,
        ) -> Result<(), ruviform_plugin_core::PrepareError> {
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            inputs: &[I],
            outputs: &mut [O],
            events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ruviform_plugin_core::ProcessContext,
        ) -> ProcessResult
        where
            I: ruviform_plugin_core::BufferView,
            O: ruviform_plugin_core::BufferViewMut,
        {
            for event in events.iter() {
                if let EventKind::ParamValue(ParamEvent { param_id, value }) = event.kind()
                    && param_id == ParamId(0)
                {
                    // Normalized [0,1] `ParamValue` → f32 is a
                    // defined narrowing within the range.
                    #[expect(clippy::cast_possible_truncation)]
                    let gain = value.get() as f32;
                    self.gain = gain;
                }
            }
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                for channel in 0..output.channels() {
                    if let (Some(src), Some(dst)) =
                        (input.channel(channel), output.channel_mut(channel))
                    {
                        for (s, d) in src.iter().zip(dst.iter_mut()) {
                            *d = *s * self.gain;
                        }
                    }
                }
            }
            ProcessResult::CONTINUE
        }
    }

    impl Plugin for L0Gain {
        fn descriptor(&self) -> PluginDescriptor {
            L0_GAIN_DESCRIPTOR
        }

        fn params(&self) -> &[ParamInfo] {
            &[]
        }

        fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
            Ok(self.gain.to_le_bytes().to_vec())
        }

        fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
            let bytes: [u8; 4] = state.try_into().map_err(|_| HostError::PluginFailed)?;
            self.gain = f32::from_le_bytes(bytes);
            Ok(())
        }
    }

    struct L0GainFactory;

    impl PluginFactory for L0GainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            L0_GAIN_DESCRIPTOR
        }

        fn create(&self) -> Box<dyn ErasedPlugin> {
            ruviform_plugin_host::erase(L0Gain { gain: 1.0 })
        }
    }

    /// A factory claiming the *worker* id — registered after an L1
    /// claim to exercise `register_factory`'s infallible shadowing.
    /// `create` is never reached: the first claimant (the L1 route)
    /// serves `Instantiate`.
    struct ShadowedWorkerFactory;

    impl PluginFactory for ShadowedWorkerFactory {
        fn descriptor(&self) -> PluginDescriptor {
            WORKER_DESCRIPTOR
        }

        fn create(&self) -> Box<dyn ErasedPlugin> {
            ruviform_plugin_host::erase(L0Gain { gain: 1.0 })
        }
    }

    /// A second factory claiming the *L0 gain* id — the L0-after-L0
    /// shadow. Its `gain` differs so `SaveState` reveals which
    /// factory `Instantiate` actually served.
    struct L0GainHalfFactory;

    impl PluginFactory for L0GainHalfFactory {
        fn descriptor(&self) -> PluginDescriptor {
            L0_GAIN_DESCRIPTOR
        }

        fn create(&self) -> Box<dyn ErasedPlugin> {
            ruviform_plugin_host::erase(L0Gain { gain: 0.5 })
        }
    }

    /// The `l1-test-worker` executable — same package, so
    /// `CARGO_BIN_EXE_*` names it directly.
    fn worker_program() -> OsString {
        OsString::from(env!("CARGO_BIN_EXE_l1-test-worker"))
    }

    /// The `ruviform-plugin-l1-runner` executable — a
    /// `ruviform-plugin-export` package bin, so `CARGO_BIN_EXE_*`
    /// cannot name it here (cargo scopes the variable to same-package
    /// bins; no cross-package compile-time mechanism exists).
    /// Resolution mirrors the #114 fixture convention
    /// (`RUVIFORM_L1_REAL_PLUGIN_DIR`): a `RUVIFORM_L1_RUNNER` env
    /// override wins verbatim, otherwise the binary is located
    /// relative to this test binary — its own directory first (the
    /// layout when the test exe sits directly in `target/<profile>/`),
    /// then `deps/`'s parent (the usual `cargo test` layout).
    fn runner_program() -> OsString {
        if let Some(path) = std::env::var_os("RUVIFORM_L1_RUNNER") {
            return path;
        }
        let exe = std::env::current_exe().expect("current_exe");
        let file_name = format!("ruviform-plugin-l1-runner{}", std::env::consts::EXE_SUFFIX);
        let candidates: Vec<PathBuf> = exe
            .parent()
            .into_iter()
            .chain(exe.parent().and_then(Path::parent))
            .map(|dir| dir.join(&file_name))
            .collect();
        for path in &candidates {
            if path.is_file() {
                return path.clone().into_os_string();
            }
        }
        panic!(
            "ruviform-plugin-l1-runner missing — searched {}; run \
             `mise run test` (workspace test builds all member bins) \
             or set RUVIFORM_L1_RUNNER",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    /// A `PluginHost` with the L0 gain factory and an L1
    /// `l1-test-worker` route registered.
    fn mixed_host(worker_args: &[&str]) -> PluginHost {
        let mut host = PluginHost::new();
        host.register_factory(Box::new(L0GainFactory));
        let mut spec = OutOfProcessSpec::new(WORKER_DESCRIPTOR, worker_program());
        spec.args = worker_args.iter().map(OsString::from).collect();
        host.register_out_of_process(spec).unwrap();
        host
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

    /// One `process` call on the composite — the public-surface
    /// mirror of `run_block`.
    fn host_block(
        host: &mut PluginHost,
        handle: PluginHandle,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<ProcessResult, HostError> {
        host_block_with_events(
            host,
            handle,
            input,
            output,
            EventList::empty(u32::try_from(input.len()).unwrap()),
        )
    }

    fn host_block_with_events(
        host: &mut PluginHost,
        handle: PluginHandle,
        input: &[f32],
        output: &mut [f32],
        events: EventList<'_>,
    ) -> Result<ProcessResult, HostError> {
        let in_list = [input];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let inputs = [HostInput(&input_view)];
        let mut out_list = [&mut output[..]];
        let mut output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut outputs = [HostOutput(&mut output_view)];
        host.process(handle, &inputs, &mut outputs, events, None)
    }

    /// Drive one instance through `Processing` — identical command
    /// series on either route.
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

    /// Polls the composite's `lifecycle_state` until `Failed`.
    fn wait_host_failed(host: &PluginHost, handle: PluginHandle) {
        let deadline = Instant::now() + NOTIFY_TIMEOUT;
        while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
            assert!(Instant::now() < deadline, "instance never reached Failed");
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Polls `try_recv_notification` until the `InstanceFailed`
    /// notification arrives or `NOTIFY_TIMEOUT` elapses.
    fn wait_host_notification(
        host: &mut PluginHost,
    ) -> (PluginHandle, ruviform_plugin_host::ChildExit) {
        let deadline = Instant::now() + NOTIFY_TIMEOUT;
        loop {
            if let Some(HostNotification::InstanceFailed { handle, exit }) =
                host.try_recv_notification()
            {
                return (handle, exit);
            }
            assert!(
                Instant::now() < deadline,
                "InstanceFailed notification never arrived"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// The mixed-route E2E (spec AC1 parity through the composite):
    /// one `PluginHost` serves an L0 factory and an L1
    /// `l1-test-worker` route behind the identical command series —
    /// scan → instantiate → lifecycle → process → state → destroy —
    /// and the caller never names a layer.
    #[test]
    fn mixed_routes_full_series() {
        let mut host = mixed_host(&[]);

        // Scan reports every registration's descriptor in order.
        let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
            panic!("unexpected reply");
        };
        assert_eq!(descriptors, vec![L0_GAIN_DESCRIPTOR, WORKER_DESCRIPTOR]);

        // The same `Instantiate{plugin_id}` shape reaches both routes.
        let l0 = instantiate(&mut host, L0_GAIN_DESCRIPTOR.id);
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert_ne!(l0, l1);
        assert_eq!(host.lifecycle_state(l0), Some(LifecycleState::Created));
        assert_eq!(host.lifecycle_state(l1), Some(LifecycleState::Created));

        // Lifecycle verbs return identical replies on both routes.
        for handle in [l0, l1] {
            drive_to_processing(&mut host, handle);
            assert_eq!(
                host.lifecycle_state(handle),
                Some(LifecycleState::Processing)
            );
        }

        // L0 `process` is the direct call: gain 1.0 → identity.
        let input = [0.75_f32; 512];
        let mut out_l0 = [0.0_f32; 512];
        assert_eq!(
            host_block(&mut host, l0, &input, &mut out_l0),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(out_l0.iter().all(|&s| sample_eq(s, 0.75)));

        // L1 `process` is the shm/try-leg path: delivered blocks are
        // exactly `0.5 ×` the input `lookahead` calls ago; underruns
        // are defined pass-through (silence/last block) and every
        // call is `Ok(CONTINUE)`.
        let mut out_l1 = [-1.0_f32; 512];
        let mut delivered = false;
        for _ in 0..16 {
            thread::sleep(Duration::from_millis(5));
            assert_eq!(
                host_block(&mut host, l1, &input, &mut out_l1),
                Ok(ProcessResult::CONTINUE)
            );
            assert!(
                out_l1
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.375)),
                "L1 output carried an undefined sample"
            );
            delivered |= out_l1.iter().all(|&s| sample_eq(s, 0.375));
        }
        assert!(delivered, "the L1 pipeline never delivered a block");

        // State round-trips on both routes through the same verbs.
        let HostReply::State { bytes } =
            host.execute(HostCommand::SaveState { handle: l0 }).unwrap()
        else {
            panic!("unexpected reply");
        };
        assert_eq!(bytes, 1.0_f32.to_le_bytes());
        host.execute(HostCommand::RestoreState {
            handle: l0,
            bytes: 0.5_f32.to_le_bytes().to_vec(),
        })
        .unwrap();
        let mut out_restored = [0.0_f32; 512];
        host_block(&mut host, l0, &input, &mut out_restored).unwrap();
        assert!(out_restored.iter().all(|&s| sample_eq(s, 0.375)));

        let HostReply::State { bytes } =
            host.execute(HostCommand::SaveState { handle: l1 }).unwrap()
        else {
            panic!("unexpected reply");
        };
        assert_eq!(bytes, [0x11, 0x22, 0x33, 0x44]);
        host.execute(HostCommand::RestoreState {
            handle: l1,
            bytes: vec![0xAA, 0xBB],
        })
        .unwrap();
        assert_eq!(
            host.execute(HostCommand::SaveState { handle: l1 }),
            Ok(HostReply::State {
                bytes: vec![0xAA, 0xBB]
            })
        );

        // `plugin_mut`: real for the L0 slot, `None` for L1 (no
        // in-process plugin object exists behind a child process).
        let plugin = host.plugin_mut(l0).expect("L0 slot has a plugin");
        assert_eq!(plugin.descriptor(), L0_GAIN_DESCRIPTOR);
        assert!(host.plugin_mut(l1).is_none());
        // GUI on L1 is the defined `Unsupported` rejection; the L0
        // fixture's default hook lands the same — for `CloseGui`
        // as much as `OpenGui`.
        for handle in [l0, l1] {
            assert_eq!(
                host.execute(HostCommand::OpenGui {
                    handle,
                    parent: GuiParentToken::new(0)
                }),
                Err(HostError::Unsupported)
            );
            assert_eq!(
                host.execute(HostCommand::CloseGui { handle }),
                Err(HostError::Unsupported)
            );
        }

        // Identical teardown on both routes; handles die with the
        // slots.
        for handle in [l0, l1] {
            host.execute(HostCommand::StopProcessing { handle })
                .unwrap();
            host.execute(HostCommand::Deactivate { handle }).unwrap();
            host.execute(HostCommand::Destroy { handle }).unwrap();
            assert_eq!(host.lifecycle_state(handle), None);
            assert_eq!(
                host.execute(HostCommand::Initialize { handle }),
                Err(HostError::InvalidHandle)
            );
        }
        // Clean teardown — nothing to notify.
        assert!(host.try_recv_notification().is_none());
    }

    /// A `plugin_id` claimed twice — in either order, across either
    /// route — is `Err(HostError::DuplicatePluginId)` on
    /// `register_out_of_process`; the first claimant keeps the route.
    #[test]
    fn duplicate_plugin_id_is_a_defined_rejection() {
        let mut host = mixed_host(&[]);

        // L1 claiming the L0 factory's id, or re-claiming its own id.
        for descriptor in [L0_GAIN_DESCRIPTOR, WORKER_DESCRIPTOR] {
            let spec = OutOfProcessSpec::new(descriptor, worker_program());
            assert_eq!(
                host.register_out_of_process(spec),
                Err(HostError::DuplicatePluginId)
            );
        }

        // The first claimant's route still serves the id — the
        // losing spec was never registered.
        let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);
        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        host.execute(HostCommand::Destroy { handle }).unwrap();
    }

    /// `register_factory` is infallible, so a `plugin_id` already
    /// claimed keeps its first route — L0-after-L1 and L0-after-L0
    /// alike. The shadowed factory stays `Scan`-listed (the
    /// composite reports every accepted registration, matching
    /// `InProcessHost`'s repeated-id semantics) while `Instantiate`
    /// honours only the first claimant.
    #[test]
    fn shadowed_factory_registration_keeps_first_claimant() {
        // L0-after-L1: the L1 claim survives a same-id factory.
        let mut host = PluginHost::new();
        host.register_out_of_process(OutOfProcessSpec::new(WORKER_DESCRIPTOR, worker_program()))
            .unwrap();
        host.register_factory(Box::new(ShadowedWorkerFactory));

        let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
            panic!("unexpected reply");
        };
        assert_eq!(descriptors, vec![WORKER_DESCRIPTOR, WORKER_DESCRIPTOR]);
        // `Instantiate` serves the L1 route — an out-of-process slot
        // has no plugin object and `SaveState` returns the worker's
        // blob, not the L0 gain's.
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert!(host.plugin_mut(l1).is_none());
        let HostReply::State { bytes } =
            host.execute(HostCommand::SaveState { handle: l1 }).unwrap()
        else {
            panic!("unexpected reply");
        };
        assert_eq!(bytes, [0x11, 0x22, 0x33, 0x44]);
        host.execute(HostCommand::Destroy { handle: l1 }).unwrap();

        // L0-after-L0: the first factory keeps serving the id —
        // `SaveState` reports the first claimant's gain, not the
        // shadow's 0.5.
        let mut host = PluginHost::new();
        host.register_factory(Box::new(L0GainFactory));
        host.register_factory(Box::new(L0GainHalfFactory));
        let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
            panic!("unexpected reply");
        };
        assert_eq!(descriptors, vec![L0_GAIN_DESCRIPTOR, L0_GAIN_DESCRIPTOR]);
        let l0 = instantiate(&mut host, L0_GAIN_DESCRIPTOR.id);
        let HostReply::State { bytes } =
            host.execute(HostCommand::SaveState { handle: l0 }).unwrap()
        else {
            panic!("unexpected reply");
        };
        assert_eq!(bytes, 1.0_f32.to_le_bytes());
        host.execute(HostCommand::Destroy { handle: l0 }).unwrap();
    }

    /// `kill_child` drives the documented crash path through the
    /// public surface: `Failed` transition → exactly one
    /// `InstanceFailed` notification carrying the public handle and
    /// the child's `ChildExit` → defined rejections afterwards.
    /// There is no automatic restart — the instance stays `Failed`.
    #[test]
    fn l1_kill_reports_failed_once() {
        let mut host = mixed_host(&[]);
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        drive_to_processing(&mut host, l1);

        host.kill_child(l1).unwrap();
        wait_host_failed(&host, l1);

        let (notified, exit) = wait_host_notification(&mut host);
        assert_eq!(notified, l1);

        // A reaped status always carries something: Windows reports
        // the kill as an exit code, unix as a signal.
        assert!(
            exit.code.is_some() || exit.signal.is_some(),
            "notification carried no exit status"
        );
        // Exactly once — the queue is already drained.
        assert!(host.try_recv_notification().is_none());

        // Defined rejections on a `Failed` instance — and no
        // automatic restart: every command keeps failing.
        assert_eq!(
            host.execute(HostCommand::Initialize { handle: l1 }),
            Err(HostError::Failed)
        );
        assert_eq!(
            host.execute(HostCommand::SaveState { handle: l1 }),
            Err(HostError::Failed)
        );
        assert_eq!(
            host.execute(HostCommand::Scan),
            Ok(HostReply::Scanned(vec![
                L0_GAIN_DESCRIPTOR,
                WORKER_DESCRIPTOR
            ]))
        );
        // `process` on the failed leg: `Ok(CONTINUE)` + silence.
        let input = [1.0_f32; 512];
        let mut output = [7.0_f32; 512];
        assert_eq!(
            host_block(&mut host, l1, &input, &mut output),
            Ok(ProcessResult::CONTINUE)
        );
        assert!(output.iter().all(|&s| sample_eq(s, 0.0)));

        // `Destroy` is the escape — `Ok`; the notification was
        // exactly once.
        host.execute(HostCommand::Destroy { handle: l1 }).unwrap();
        assert_eq!(host.lifecycle_state(l1), None);
        assert!(host.try_recv_notification().is_none());
    }

    /// The L0 legality rule is enforced at the composite for L1 too:
    /// `Destroy` from `Activated`/`Processing` is
    /// `Err(HostError::Lifecycle(_))` — the child is never told
    /// (`L1Instance::destroy` is unconditional; the gate lives here).
    #[test]
    fn l1_destroy_honours_lifecycle_legality() {
        let mut host = mixed_host(&[]);
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        host.execute(HostCommand::Initialize { handle: l1 })
            .unwrap();
        host.execute(HostCommand::Activate {
            handle: l1,
            config: config(),
        })
        .unwrap();
        assert!(matches!(
            host.execute(HostCommand::Destroy { handle: l1 }),
            Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: LifecycleState::Activated,
                event: LifecycleEvent::Destroy,
            }))
        ));
        // The rejection left the machine untouched.
        assert_eq!(host.lifecycle_state(l1), Some(LifecycleState::Activated));

        host.execute(HostCommand::StartProcessing { handle: l1 })
            .unwrap();
        assert!(matches!(
            host.execute(HostCommand::Destroy { handle: l1 }),
            Err(HostError::Lifecycle(LifecycleError::IllegalTransition {
                from: LifecycleState::Processing,
                event: LifecycleEvent::Destroy,
            }))
        ));

        // Legal path: back down to `Initialized`, then `Destroy → Ok`.
        host.execute(HostCommand::StopProcessing { handle: l1 })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle: l1 })
            .unwrap();
        host.execute(HostCommand::Destroy { handle: l1 }).unwrap();
        assert_eq!(host.lifecycle_state(l1), None);
    }

    /// `StopProcessing` is the one commit-then-advisory verb: the
    /// child applies `Processing → Activated` *before* the
    /// `stop_processing()` hook can decline (L0 semantics exactly),
    /// so a hook `Err` surfaces post-commit — `execute` reports the
    /// `Err` while `lifecycle_state()` must already read
    /// `Activated`. `--fail-stop-processing` drives that shape.
    #[test]
    fn l1_stop_processing_advisory_err_still_reports_activated() {
        let mut host = mixed_host(&["--fail-stop-processing"]);
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        drive_to_processing(&mut host, l1);

        assert_eq!(
            host.execute(HostCommand::StopProcessing { handle: l1 }),
            Err(HostError::PluginFailed)
        );
        // The child already left `Processing` — a lagging mirror
        // would keep reporting `Processing` and keep doorbelling.
        assert_eq!(host.lifecycle_state(l1), Some(LifecycleState::Activated));
        // The RT leg agrees: outside `Processing` it is the same
        // `Err(Lifecycle)` L0 returns, never a doorbell.
        let input = [1.0_f32; 512];
        let mut output = [0.0_f32; 512];
        assert!(matches!(
            host_block(&mut host, l1, &input, &mut output),
            Err(HostError::Lifecycle(_))
        ));
        // Child-side legality is the oracle: `StartProcessing` is
        // legal from `Activated` (illegal from `Processing`), so the
        // child really is where the mirror claims.
        host.execute(HostCommand::StartProcessing { handle: l1 })
            .unwrap();
        assert_eq!(host.lifecycle_state(l1), Some(LifecycleState::Processing));
        // The flag is persistent — the second stop repeats the
        // commit-then-advisory shape.
        assert_eq!(
            host.execute(HostCommand::StopProcessing { handle: l1 }),
            Err(HostError::PluginFailed)
        );
        assert_eq!(host.lifecycle_state(l1), Some(LifecycleState::Activated));

        host.execute(HostCommand::Deactivate { handle: l1 })
            .unwrap();
        host.execute(HostCommand::Destroy { handle: l1 }).unwrap();
        assert_eq!(host.lifecycle_state(l1), None);
    }

    /// Unified-handle hygiene: destroyed slots are reused with a
    /// bumped generation and stale handles can never cross-route —
    /// a dead L0 handle stays dead even when an L1 instance occupies
    /// the slot it was minted from (and vice versa).
    #[test]
    fn stale_handles_never_cross_route() {
        let mut host = mixed_host(&[]);
        let l0 = instantiate(&mut host, L0_GAIN_DESCRIPTOR.id);
        host.execute(HostCommand::Initialize { handle: l0 })
            .unwrap();
        host.execute(HostCommand::Destroy { handle: l0 }).unwrap();
        assert_eq!(
            host.execute(HostCommand::Initialize { handle: l0 }),
            Err(HostError::InvalidHandle)
        );
        assert!(host.plugin_mut(l0).is_none());
        assert_eq!(host.kill_child(l0), Err(HostError::InvalidHandle));

        // The freed slot is reused — by *either* route — with a
        // bumped generation, and the stale L0 handle cannot reach it.
        let l1 = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert_eq!(l1.slot(), l0.slot());
        assert_eq!(l1.generation(), l0.generation() + 1);
        assert_eq!(
            host.execute(HostCommand::Initialize { handle: l0 }),
            Err(HostError::InvalidHandle)
        );
        host.execute(HostCommand::Initialize { handle: l1 })
            .unwrap();
        host.execute(HostCommand::Destroy { handle: l1 }).unwrap();

        // The slot re-fills on the L0 route too; the stale L1 handle
        // stays dead, and `kill_child` on a live L0 slot is the
        // defined `Unsupported` (no child exists).
        let l0b = instantiate(&mut host, L0_GAIN_DESCRIPTOR.id);
        assert_eq!(
            host.execute(HostCommand::Initialize { handle: l1 }),
            Err(HostError::InvalidHandle)
        );
        assert_eq!(host.kill_child(l0b), Err(HostError::Unsupported));
        host.execute(HostCommand::Destroy { handle: l0b }).unwrap();
    }

    /// The real `ruviform-plugin-l1-runner` bin behind
    /// `OutOfProcessSpec.program` — the export-side conjunction:
    /// spawn → handshake (descriptor match against the runner's
    /// built-in `GainPlugin`) → lifecycle → shm block processing with
    /// the param event → state round-trip → `Destroy`.
    #[test]
    fn runner_bin_serves_real_gain_plugin() {
        let mut host = PluginHost::new();
        host.register_out_of_process(OutOfProcessSpec::new(
            RUNNER_GAIN_DESCRIPTOR,
            runner_program(),
        ))
        .unwrap();

        let HostReply::Scanned(descriptors) = host.execute(HostCommand::Scan).unwrap() else {
            panic!("unexpected reply");
        };
        assert_eq!(descriptors, vec![RUNNER_GAIN_DESCRIPTOR]);

        let handle = instantiate(&mut host, RUNNER_GAIN_DESCRIPTOR.id);
        drive_to_processing(&mut host, handle);

        // ParamValue 0.25 → denormalized over [0, 2.0] → gain 0.5:
        // delivered blocks carry `0.5 ×` an earlier input exactly;
        // underruns are defined pass-through.
        let event = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.25).unwrap(),
            }),
        );
        let events = [event];
        let event_list = EventList::new(&events, 512).unwrap();
        let mut inputs: Vec<f32> = Vec::new();
        let mut delivered = false;
        for call in 0..32_u32 {
            let value = 0.2 + 0.01 * f32::from(u8::try_from(call % 16).unwrap() + 1);
            let input = vec![value; 512];
            let mut output = vec![0.0_f32; 512];
            thread::sleep(Duration::from_millis(5));
            assert_eq!(
                host_block_with_events(&mut host, handle, &input, &mut output, event_list),
                Ok(ProcessResult::CONTINUE)
            );
            if output.iter().all(|&s| sample_eq(s, 0.0)) {
                // Defined underrun silence.
            } else {
                // A delivered block is exactly `0.5 ×` some earlier
                // input (the runner applies the param gain).
                assert!(
                    output
                        .iter()
                        .all(|&s| inputs.iter().any(|&v| sample_eq(s, 0.5 * v))),
                    "call {call}: undefined output sample {}",
                    output[0]
                );
                delivered = true;
            }
            inputs.push(value);
        }
        assert!(delivered, "the runner pipeline never delivered");

        // State round-trip through the wire: `RFG1` + f32 gain.
        let HostReply::State { bytes } = host.execute(HostCommand::SaveState { handle }).unwrap()
        else {
            panic!("unexpected reply");
        };
        assert_eq!(&bytes[..4], b"RFG1");
        assert_eq!(bytes[4..], 0.5_f32.to_le_bytes());
        host.execute(HostCommand::RestoreState {
            handle,
            bytes: [b"RFG1".as_slice(), &0.75_f32.to_le_bytes()].concat(),
        })
        .unwrap();

        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        host.execute(HostCommand::Destroy { handle }).unwrap();
        assert_eq!(host.lifecycle_state(handle), None);
    }

    // --------------------------------------------------------------
    // Issue #117 `Bundled` route E2E — the pool selects one child
    // for every `Instantiate` that fits; the rows below pin the
    // whole contract through the public `PluginHost` surface.
    // --------------------------------------------------------------

    /// The pooled-route contract end to end: two `Instantiate`s on
    /// the default `Bundled` route land on *one* child — proven
    /// operationally by the kill fan-out at the end — with distinct
    /// public handles, independent lifecycle (the child's own
    /// per-instance legality agrees with each mirror), independent
    /// state blobs, and interleaved data planes that never cross
    /// (disjoint input ranges make a foreign block an undefined
    /// sample). One `kill_child` then converges *both* `Failed`
    /// and emits one `InstanceFailed` per minted handle carrying
    /// the same `ChildExit`.
    #[test]
    fn bundled_route_shares_one_child_across_instances() {
        let mut host = mixed_host(&[]);
        let a = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        let b = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert_ne!(a, b);
        assert_eq!(host.lifecycle_state(a), Some(LifecycleState::Created));
        assert_eq!(host.lifecycle_state(b), Some(LifecycleState::Created));

        // Lifecycle independence: `a` walks to `Processing` while
        // `b` stays `Created` — and the child's per-instance
        // legality agrees (`Activate` on `b` is the defined
        // `Err(Lifecycle)`, not a child-scope violation).
        drive_to_processing(&mut host, a);
        assert_eq!(host.lifecycle_state(b), Some(LifecycleState::Created));
        assert!(matches!(
            host.execute(HostCommand::Activate {
                handle: b,
                config: config(),
            }),
            Err(HostError::Lifecycle(_))
        ));
        drive_to_processing(&mut host, b);

        // Per-instance state blobs — `a`'s `RestoreState` does not
        // touch `b`'s.
        host.execute(HostCommand::RestoreState {
            handle: a,
            bytes: vec![0xAA],
        })
        .unwrap();
        assert_eq!(
            host.execute(HostCommand::SaveState { handle: b }),
            Ok(HostReply::State {
                bytes: vec![0x11, 0x22, 0x33, 0x44]
            })
        );

        // Interleaved processing on disjoint input ranges — the
        // `(instance, seq)` correlation oracle at the data plane:
        // any reply or shm segment crossing instances lands an
        // undefined sample.
        let mut a_inputs: Vec<f32> = Vec::new();
        let mut b_inputs: Vec<f32> = Vec::new();
        let mut delivered_a = false;
        let mut delivered_b = false;
        for call in 0..24_u32 {
            let va = 0.2 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
            let vb = 5.0 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
            let input_a = vec![va; 512];
            let input_b = vec![vb; 512];
            let mut out_a = vec![0.0_f32; 512];
            let mut out_b = vec![0.0_f32; 512];
            thread::sleep(Duration::from_millis(5));
            assert_eq!(
                host_block(&mut host, a, &input_a, &mut out_a),
                Ok(ProcessResult::CONTINUE)
            );
            assert_eq!(
                host_block(&mut host, b, &input_b, &mut out_b),
                Ok(ProcessResult::CONTINUE)
            );
            assert!(
                out_a
                    .iter()
                    .all(|&s| super::is_defined_sample(s, &a_inputs)),
                "call {call}: a carried a foreign/undefined sample {}",
                out_a[0]
            );
            assert!(
                out_b
                    .iter()
                    .all(|&s| super::is_defined_sample(s, &b_inputs)),
                "call {call}: b carried a foreign/undefined sample {}",
                out_b[0]
            );
            delivered_a |= call >= 2
                && out_a
                    .iter()
                    .all(|&s| sample_eq(s, 0.5 * a_inputs[call as usize - 2]));
            delivered_b |= call >= 2
                && out_b
                    .iter()
                    .all(|&s| sample_eq(s, 0.5 * b_inputs[call as usize - 2]));
            a_inputs.push(va);
            b_inputs.push(vb);
        }
        assert!(delivered_a, "a's pipeline never delivered");
        assert!(delivered_b, "b's pipeline never delivered");

        // The child-scope oracle: one `kill_child` converges every
        // co-resident instance — under `Bundled` both handles are on
        // the same child, so both go `Failed` and each minted
        // handle gets exactly one `InstanceFailed` with the same
        // exit status.
        host.kill_child(a).unwrap();
        wait_host_failed(&host, a);
        wait_host_failed(&host, b);
        let mut notified = [
            wait_host_notification(&mut host),
            wait_host_notification(&mut host),
        ];
        notified.sort_by_key(|(handle, _)| handle.slot());
        assert_eq!(notified[0].0, a);
        assert_eq!(notified[1].0, b);
        assert_eq!(notified[0].1, notified[1].1);
        assert!(host.try_recv_notification().is_none());

        // Both stay `Failed` until `Destroy` — no restart.
        for handle in [a, b] {
            assert_eq!(
                host.execute(HostCommand::Initialize { handle }),
                Err(HostError::Failed)
            );
            host.execute(HostCommand::Destroy { handle }).unwrap();
            assert_eq!(host.lifecycle_state(handle), None);
        }
    }

    /// Shutdown-on-empty through the public route: the first
    /// `Destroy` reclaims its slot but keeps the shared child
    /// serving the sibling (`live_count != 0` → no eviction); the
    /// last `Destroy` evicts the emptied child and runs the
    /// `Shutdown`+grace+reap teardown — then the next `Instantiate`
    /// pays a fresh spawn and works normally. A clean teardown
    /// never notifies.
    #[test]
    fn bundled_last_destroy_shuts_down_child_and_respawns() {
        let mut host = mixed_host(&[]);
        let a = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        let b = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        host.execute(HostCommand::Initialize { handle: a }).unwrap();
        host.execute(HostCommand::Initialize { handle: b }).unwrap();

        // Mid-bundle destroy: `b` keeps the child alive — the pool
        // does not evict a child with live siblings.
        host.execute(HostCommand::Destroy { handle: a }).unwrap();
        assert_eq!(host.lifecycle_state(a), None);
        assert_eq!(host.lifecycle_state(b), Some(LifecycleState::Initialized));
        // `b` still serves — the child never saw `Shutdown` for `a`.
        assert_eq!(
            host.execute(HostCommand::SaveState { handle: b }),
            Ok(HostReply::State {
                bytes: vec![0x11, 0x22, 0x33, 0x44]
            })
        );

        // The last `Destroy` empties the registry → the pool evicts
        // and tears the child down inside this same call (bounded:
        // `Shutdown` + grace + reap + join). Clean — nothing to
        // notify.
        host.execute(HostCommand::Destroy { handle: b }).unwrap();
        assert_eq!(host.lifecycle_state(b), None);
        assert!(host.try_recv_notification().is_none());

        // The route recovered: the next `Instantiate` spawns fresh
        // (the dead pool entry was evicted, not re-served).
        let c = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        drive_to_processing(&mut host, c);
        host.execute(HostCommand::StopProcessing { handle: c })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle: c }).unwrap();
        host.execute(HostCommand::Destroy { handle: c }).unwrap();
        assert!(host.try_recv_notification().is_none());
    }

    /// Mid-`CreateInstance` death: the second `Instantiate` reuses
    /// the pooled child, whose crash knob fires on receipt of the
    /// third parent→child message (`Scan`, `CreateInstance` ×2) —
    /// the `CreateInstance` waiter is woken `Err(Ipc)`, no facade is
    /// minted (no `Failed` notification for a handle that never
    /// existed), the converged child is evicted, and the minted
    /// sibling fans out its own `Failed` + exactly one
    /// notification. The route then recovers on a fresh child.
    #[test]
    fn bundled_mid_create_death_notifies_only_minted() {
        let mut host = mixed_host(&["--crash-after-msgs=3"]);
        let a = instantiate(&mut host, WORKER_DESCRIPTOR.id);

        // The reused child's `CreateInstance` is the crash
        // message — `Instantiate` surfaces `Err(Ipc)`, no handle.
        assert_eq!(
            host.execute(HostCommand::Instantiate {
                plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
            }),
            Err(HostError::Ipc)
        );

        // The minted instance converged `Failed` by the child-scope
        // fan-out — exactly one `InstanceFailed`, carrying `a`'s
        // handle. The never-minted `B` emits nothing.
        wait_host_failed(&host, a);
        let (notified, exit) = wait_host_notification(&mut host);
        assert_eq!(notified, a);
        assert!(exit.code.is_some() || exit.signal.is_some());
        assert!(host.try_recv_notification().is_none());

        host.execute(HostCommand::Destroy { handle: a }).unwrap();
        // The route recovered — the evicted child's replacement
        // spawns and mints `c` normally. The crash knob is
        // per-child, so `c`'s `DestroyInstance` is *its* third
        // message: `Destroy` re-runs the mid-destroy death path and
        // still frees the slot `Ok` (its notification leaves with
        // the freed slot, exactly like `a`'s above).
        let c = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert_eq!(
            host.execute(HostCommand::Destroy { handle: c }),
            Ok(HostReply::Ok)
        );
        assert_eq!(host.lifecycle_state(c), None);
        assert!(host.try_recv_notification().is_none());
    }

    /// Mid-`DestroyInstance` death: `Destroy` is unconditional once
    /// the lifecycle gate passes — the child dying on the request
    /// (`--crash-after-msgs=4`: `Scan`, `CreateInstance` ×2, the
    /// `DestroyInstance`) still reports `Ok`, the slot frees, and
    /// the sibling converges `Failed` by the same fan-out with its
    /// own notification (the destroyed instance's own notification
    /// leaves with its slot — `b`'s is the observable one).
    #[test]
    fn bundled_mid_destroy_death_ok_and_sibling_failed() {
        let mut host = mixed_host(&["--crash-after-msgs=4"]);
        let a = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        let b = instantiate(&mut host, WORKER_DESCRIPTOR.id);

        // `a`'s `DestroyInstance` is the crash message — the
        // request fails, but `Destroy` still frees the slot `Ok`.
        assert_eq!(
            host.execute(HostCommand::Destroy { handle: a }),
            Ok(HostReply::Ok)
        );
        assert_eq!(host.lifecycle_state(a), None);

        // `b` converged `Failed` on the same child's death — one
        // notification carrying `b`'s handle and the real exit.
        wait_host_failed(&host, b);
        let (notified, exit) = wait_host_notification(&mut host);
        assert_eq!(notified, b);
        assert!(exit.code.is_some() || exit.signal.is_some());
        assert!(host.try_recv_notification().is_none());
        host.execute(HostCommand::Destroy { handle: b }).unwrap();
    }

    /// `Dedicated` is the capacity-1 degeneration of the same
    /// machinery: two `Instantiate`s get one child *each*, so
    /// killing `a`'s child converges `a` alone — `b` stays
    /// `Processing` and keeps delivering blocks. The non-regression
    /// pin the `Bundled` fan-out tests are the dual of.
    #[test]
    fn dedicated_route_isolates_instances_per_child() {
        let mut host = PluginHost::new();
        let mut spec = OutOfProcessSpec::new(WORKER_DESCRIPTOR, worker_program());
        spec.isolation = OutOfProcessIsolation::Dedicated;
        host.register_out_of_process(spec).unwrap();

        let a = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        let b = instantiate(&mut host, WORKER_DESCRIPTOR.id);
        assert_ne!(a, b);
        drive_to_processing(&mut host, a);
        drive_to_processing(&mut host, b);

        host.kill_child(a).unwrap();
        wait_host_failed(&host, a);
        let (notified, exit) = wait_host_notification(&mut host);
        assert_eq!(notified, a);
        assert!(exit.code.is_some() || exit.signal.is_some());
        assert!(host.try_recv_notification().is_none());

        // `b`'s child is a different process — untouched.
        assert_eq!(host.lifecycle_state(b), Some(LifecycleState::Processing));
        let input = [1.0_f32; 512];
        let mut output = [0.0_f32; 512];
        let mut delivered = false;
        for _ in 0..16 {
            thread::sleep(Duration::from_millis(5));
            assert_eq!(
                host_block(&mut host, b, &input, &mut output),
                Ok(ProcessResult::CONTINUE)
            );
            assert!(
                output
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)),
                "b emitted an undefined sample after a's child died"
            );
            delivered |= output.iter().all(|&s| sample_eq(s, 0.5));
        }
        assert!(delivered, "b's dedicated child stopped delivering");

        host.execute(HostCommand::Destroy { handle: a }).unwrap();
        host.execute(HostCommand::StopProcessing { handle: b })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle: b }).unwrap();
        host.execute(HostCommand::Destroy { handle: b }).unwrap();
    }

    /// The real `ruviform-plugin-l1-runner` under the `Bundled`
    /// route: two `Instantiate`s land on one runner child whose
    /// per-instance registry keeps lifecycle, param state and shm
    /// processing independent — `a`'s param event sets gain 0.5,
    /// `b`'s sets 0.75, and delivered blocks carry only the owning
    /// instance's multiplier — then one `kill_child` fans `Failed`
    /// out to both with the same exit status.
    #[test]
    fn runner_bin_serves_bundled_gain_instances() {
        let mut host = PluginHost::new();
        host.register_out_of_process(OutOfProcessSpec::new(
            RUNNER_GAIN_DESCRIPTOR,
            runner_program(),
        ))
        .unwrap();
        let a = instantiate(&mut host, RUNNER_GAIN_DESCRIPTOR.id);
        let b = instantiate(&mut host, RUNNER_GAIN_DESCRIPTOR.id);
        assert_ne!(a, b);
        drive_to_processing(&mut host, a);
        drive_to_processing(&mut host, b);

        // Per-instance param events on the shared child: 0.25 →
        // gain 0.5 on `a`, 0.375 → 0.75 on `b` (normalized [0,1]
        // denormalized over the gain range [0,2]).
        let events_a = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.25).unwrap(),
            }),
        )];
        let events_b = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.375).unwrap(),
            }),
        )];
        let events_a = EventList::new(&events_a, 512).unwrap();
        let events_b = EventList::new(&events_b, 512).unwrap();
        let input_a = [1.0_f32; 512];
        let input_b = [2.0_f32; 512];
        let mut out_a = [0.0_f32; 512];
        let mut out_b = [0.0_f32; 512];
        let mut delivered_a = false;
        let mut delivered_b = false;
        for _ in 0..24 {
            thread::sleep(Duration::from_millis(5));
            assert_eq!(
                host_block_with_events(&mut host, a, &input_a, &mut out_a, events_a),
                Ok(ProcessResult::CONTINUE)
            );
            assert_eq!(
                host_block_with_events(&mut host, b, &input_b, &mut out_b, events_b),
                Ok(ProcessResult::CONTINUE)
            );
            // Defined sets are per-instance: `a` yields silence or
            // `0.5 × 1.0`, `b` silence or `0.75 × 2.0` — the wrong
            // multiplier or a foreign input is an undefined sample.
            assert!(
                out_a
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)),
                "a carried a foreign/undefined sample"
            );
            assert!(
                out_b
                    .iter()
                    .all(|&s| sample_eq(s, 0.0) || sample_eq(s, 1.5)),
                "b carried a foreign/undefined sample"
            );
            delivered_a |= out_a.iter().all(|&s| sample_eq(s, 0.5));
            delivered_b |= out_b.iter().all(|&s| sample_eq(s, 1.5));
        }
        assert!(delivered_a, "a's runner pipeline never delivered");
        assert!(delivered_b, "b's runner pipeline never delivered");

        // Child-scoped kill: both instances on the one runner child
        // converge `Failed` — two `InstanceFailed`s, same exit.
        host.kill_child(a).unwrap();
        wait_host_failed(&host, a);
        wait_host_failed(&host, b);
        let mut notified = [
            wait_host_notification(&mut host),
            wait_host_notification(&mut host),
        ];
        notified.sort_by_key(|(handle, _)| handle.slot());
        assert_eq!(notified[0].0, a);
        assert_eq!(notified[1].0, b);
        assert_eq!(notified[0].1, notified[1].1);
        assert!(host.try_recv_notification().is_none());
        host.execute(HostCommand::Destroy { handle: a }).unwrap();
        host.execute(HostCommand::Destroy { handle: b }).unwrap();
    }
}
