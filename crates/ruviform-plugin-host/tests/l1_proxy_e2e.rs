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
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::SampleRate;
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    GuiParentToken, HostError, LifecycleState, PluginDescriptor, PrepareError, ProcessConfig,
};

use l1::{ChildExit, L1Instance, SpawnSpec};

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
/// on the 3rd received message — the handshake `Scan` is message 1 —
/// so `Activate`'s pending waiter is woken `Err(Ipc)`, the instance
/// converges `Failed` with exactly one notification, and every later
/// command except `Destroy` is rejected.
#[test]
fn crash_transitions_failed_and_notifies() {
    let mut instance = L1Instance::spawn(&spec(&["--crash-after-msgs=3"])).unwrap();
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
        L1Instance::spawn(&spec(&["--crash-after-msgs=2", "--crash-exit-code=0"])).unwrap();

    // Message 2 (`Initialize`) kills the worker mid-command.
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
    let mut instance = L1Instance::spawn(&fast_timeout_spec(&["--hang-after-msgs=1"])).unwrap();

    // `Scan` (handshake) was message 1 — `Initialize` is never answered.
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

/// Wedged-child teardown (A1): after `--stop-reading-after-msgs=2`
/// the child never drains stdin again, so a `RestoreState` frame far
/// over the pipe capacity leaves the driver blocked in `write_all`.
/// The `command_timeout` convergence must itself kill the child —
/// that is what turns the wedged write into EPIPE and lets the
/// driver unwind — and destroy/drop afterwards stay bounded.
#[test]
fn wedged_child_write_converges_and_teardown_is_bounded() {
    let mut instance =
        L1Instance::spawn(&fast_timeout_spec(&["--stop-reading-after-msgs=2"])).unwrap();
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

/// Detached reader (A4): `--leave-heir-after-msgs=1` makes the worker
/// answer the handshake `Scan`, then spawn a stdio-inheriting
/// grandchild and exit — the parent's stdout pipe stays open, so the
/// reader thread never observes EOF. `try_wait` still converges
/// `Failed`, and destroy/drop must detach the blocked reader rather
/// than join it.
#[test]
fn destroy_detaches_reader_when_heir_holds_stdout() {
    let mut instance = L1Instance::spawn(&spec(&["--leave-heir-after-msgs=1"])).unwrap();

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
