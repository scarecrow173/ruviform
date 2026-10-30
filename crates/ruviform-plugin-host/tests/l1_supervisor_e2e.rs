//! L1 supervisor E2E (Issue #116 task T3 —
//! `docs/tasks/l1-supervisor-e2e.md`, oracle
//! `docs/test-specs/issue-116-supervisor-e2e-tests.md`).
//!
//! Drives the opt-in `Supervisor` against real `l1-test-worker`
//! child processes (`env!("CARGO_BIN_EXE_l1-test-worker")`) through
//! `PluginHost`'s public surface only — `execute`,
//! `lifecycle_state`, `try_recv_notification`, `prepare_endpoint`,
//! the `#[doc(hidden)]` `kill_child` crash knob and the worker's
//! argv fault flags.
//!
//! **No `#[path]` harness:** unlike `l1_proxy_e2e.rs` this is a
//! separate integration-test target importing only public API —
//! `pub(crate)` L1 machinery (`l1::`, `Route`, `Slot`, pool
//! internals) cannot resolve here, which is itself the
//! compile-level AC1 proof that the supervisor needs nothing beyond
//! the public surface.

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC_DISABLER: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

use std::ffi::OsString;
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio_core::{AudioBufferViewMut, SampleRate};
use ruviform_plugin_core::wire;
use ruviform_plugin_core::{
    EventList, FailureCause, HostError, LifecycleState, PluginDescriptor, PrepareError,
    ProcessConfig, ProcessResult,
};
use ruviform_plugin_host::{
    ChildExit, DiagnosticQuery, EndpointBounds, HostCommand, HostNotification, HostReply,
    OutOfProcessIsolation, OutOfProcessSpec, PluginHandle, PluginHost, RestartPolicy,
    RestartRecipe, RestoreOutcome, SupervisedId, SupervisionError, SupervisionEvent, Supervisor,
};
use ruviform_rt::RtContext;
use static_assertions::{assert_impl_all, assert_not_impl_any};

// Type witnesses (spec AC8): the supervisor carries only sendable
// bookkeeping (`usize` binding, plain data) and composes with the
// host's deliberately `!Send`/`!Sync` ownership — `poll(&mut host,
// now)` is driven synchronously; no thread, timer or async runtime
// lives inside it.
assert_impl_all!(Supervisor: Send, Sync);
assert_not_impl_any!(PluginHost: Send, Sync);

/// The descriptor `l1-test-worker` reports for `Scan` — the same
/// value `l1_proxy_e2e` pins (`SpawnSpec`/`OutOfProcessSpec` must
/// field-match it).
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// How long tests poll for asynchronous `Failed` convergence and
/// supervision events — crash detection needs at most a driver poll
/// plus scheduling; 10 s is generous headroom for loaded CI hosts
/// (mirrors `l1_proxy_e2e`'s `NOTIFY_TIMEOUT`).
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// Byte overhead one wire command frame adds around its payload —
/// `seq` 8 + `instance` 4 + `verb` 1 + `len` 4 = 17 B. The
/// `set_checkpoint` bound keeps a `RestoreState` built from a
/// registered blob inside one `WIRE_MAX_PAYLOAD` frame.
const WIRE_COMMAND_ENVELOPE: usize = 17;

fn worker_spec(args: &[&str]) -> OutOfProcessSpec {
    let mut spec = OutOfProcessSpec::new(
        WORKER_DESCRIPTOR,
        OsString::from(env!("CARGO_BIN_EXE_l1-test-worker")),
    );
    spec.args = args.iter().map(OsString::from).collect();
    spec
}

/// A `PluginHost` with only the worker's L1 route registered —
/// `Bundled` (the default) or `Dedicated` per `isolation`.
fn worker_host(args: &[&str], isolation: OutOfProcessIsolation) -> PluginHost {
    let mut spec = worker_spec(args);
    spec.isolation = isolation;
    let mut host = PluginHost::new();
    host.register_out_of_process(spec).unwrap();
    host
}

fn bundled_host(args: &[&str]) -> PluginHost {
    worker_host(args, OutOfProcessIsolation::Bundled)
}

fn dedicated_host(args: &[&str]) -> PluginHost {
    worker_host(args, OutOfProcessIsolation::Dedicated)
}

fn config() -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
}

fn bounds() -> EndpointBounds {
    EndpointBounds {
        max_frames: 512,
        channels: 2,
    }
}

/// `config: None` — recovers to `Initialized` (the recipe's
/// `endpoint_bounds` echo is dead data there).
fn minimal_recipe() -> RestartRecipe {
    RestartRecipe {
        plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
        config: None,
        start_processing: false,
        endpoint_bounds: Some(bounds()),
    }
}

/// `config: Some` — recovers to `Activated` (endpoint-attachable).
fn activated_recipe() -> RestartRecipe {
    RestartRecipe {
        config: Some(config()),
        ..minimal_recipe()
    }
}

/// `config: Some` + `start_processing` — recovers to `Processing`.
fn processing_recipe() -> RestartRecipe {
    RestartRecipe {
        start_processing: true,
        ..activated_recipe()
    }
}

fn instantiate(host: &mut PluginHost) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: WORKER_DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// Caller-side lifecycle drive for fixture setup: `Initialize`,
/// then `Activate`/`StartProcessing` per `config`/`start` — the
/// same series the restart replay issues.
fn drive(host: &mut PluginHost, handle: PluginHandle, config: Option<ProcessConfig>, start: bool) {
    host.execute(HostCommand::Initialize { handle }).unwrap();
    if let Some(config) = config {
        host.execute(HostCommand::Activate { handle, config })
            .unwrap();
        if start {
            host.execute(HostCommand::StartProcessing { handle })
                .unwrap();
        }
    }
}

/// Caller-side teardown — `Destroy` is legal only from `Created`,
/// `Initialized` or `Failed`, so unwind `Processing`/`Activated`
/// first.
fn destroy(host: &mut PluginHost, handle: PluginHandle) {
    match host.lifecycle_state(handle) {
        Some(LifecycleState::Processing) => {
            host.execute(HostCommand::StopProcessing { handle })
                .unwrap();
            host.execute(HostCommand::Deactivate { handle }).unwrap();
        }
        Some(LifecycleState::Activated) => {
            host.execute(HostCommand::Deactivate { handle }).unwrap();
        }
        _ => {}
    }
    host.execute(HostCommand::Destroy { handle }).unwrap();
}

/// Polls `lifecycle_state` until the instance converges `Failed`
/// (the L1 driver's asynchronous detection).
fn wait_for_failed(host: &PluginHost, handle: PluginHandle) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls `try_recv_notification` until one `InstanceFailed`
/// notification arrives — drains it for the caller (the adopt
/// variant where the notification was already consumed).
fn wait_notification(host: &mut PluginHost) -> HostNotification {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    loop {
        if let Some(notification) = host.try_recv_notification() {
            return notification;
        }
        assert!(
            Instant::now() < deadline,
            "InstanceFailed notification never arrived"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// Lets the driver's reap complete a `Pending → Sent` notification:
/// `wait_for_failed` only observes the mirror, which converges one
/// driver poll (~200 µs) before the exit status completes the
/// notification. Only needed where a later `poll` phase could
/// preempt the drain — multi-notification fan-outs and the
/// lingering `GaveUp` re-emit.
fn settle_notification() {
    thread::sleep(Duration::from_millis(50));
}

/// Calls `sup.poll(host, now)` until `want` holds on the
/// accumulated events — notifications arrive asynchronously after
/// `kill_child`, so the drain needs bounded retries. `now` is the
/// injected `Instant` every call sees (non-decreasing per
/// supervisor is the caller contract — backoff tests pass derived
/// offsets of one base).
fn poll_until_event(
    sup: &mut Supervisor,
    host: &mut PluginHost,
    now: Instant,
    mut want: impl FnMut(&[SupervisionEvent]) -> bool,
) -> Vec<SupervisionEvent> {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    let mut events = Vec::new();
    loop {
        events.extend(sup.poll(host, now));
        if want(&events) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "supervision events never arrived: {events:?}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls until `current_handle(id)` returns `None` — the residue
/// clear paths (Phase-2 retry `Destroy`, drained lingering match
/// and external-`Destroy` observation) emit no event.
fn poll_until_cleared(sup: &mut Supervisor, host: &mut PluginHost, now: Instant, id: SupervisedId) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while sup.current_handle(id).is_some() {
        let _ = sup.poll(host, now);
        assert!(Instant::now() < deadline, "lingering residue never cleared");
        thread::sleep(Duration::from_millis(5));
    }
}

fn restarted(
    events: &[SupervisionEvent],
    id: SupervisedId,
) -> Option<(
    PluginHandle,
    PluginHandle,
    Option<EndpointBounds>,
    RestoreOutcome,
)> {
    events.iter().find_map(|event| match event {
        SupervisionEvent::Restarted {
            id: hit,
            old,
            new,
            bounds,
            restored,
            ..
        } if *hit == id => Some((*old, *new, *bounds, *restored)),
        _ => None,
    })
}

fn count_restarted(events: &[SupervisionEvent], id: SupervisedId) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, SupervisionEvent::Restarted { id: hit, .. } if *hit == id))
        .count()
}

fn attempt_faileds(events: &[SupervisionEvent], id: SupervisedId) -> Vec<(u32, HostError)> {
    events
        .iter()
        .filter_map(|event| match event {
            SupervisionEvent::AttemptFailed {
                id: hit,
                attempt,
                error,
                ..
            } if *hit == id => Some((*attempt, *error)),
            _ => None,
        })
        .collect()
}

/// A pre-mint attempt error is `SpawnFailed` — the cause is the
/// driver-observed convergence (exit or broken pipe, whichever
/// raced first) — with a `diagnostics_id` queryable `Found` on the
/// host's registry.
fn expect_spawn_failed(host: &PluginHost, error: HostError) {
    let HostError::SpawnFailed {
        cause,
        diagnostics_id,
    } = error
    else {
        panic!("expected SpawnFailed, got {error:?}")
    };
    assert!(matches!(
        cause,
        FailureCause::ExitObserved | FailureCause::Transport
    ));
    assert!(matches!(
        host.diagnostics_query(diagnostics_id),
        DiagnosticQuery::Found(_)
    ));
}

fn gave_ups(events: &[SupervisionEvent], id: SupervisedId) -> Vec<(u32, Option<HostError>)> {
    events
        .iter()
        .filter_map(|event| match event {
            SupervisionEvent::GaveUp {
                id: hit,
                attempts,
                last_error,
                ..
            } if *hit == id => Some((*attempts, *last_error)),
            _ => None,
        })
        .collect()
}

/// The `(handle, exit, cause)` of every `Unsupervised`
/// pass-through — verbatim `HostNotification` payloads, cause
/// included.
fn unsupervised_failures(
    events: &[SupervisionEvent],
) -> Vec<(PluginHandle, ChildExit, FailureCause)> {
    events
        .iter()
        .filter_map(|event| match event {
            SupervisionEvent::Unsupervised {
                notification:
                    HostNotification::InstanceFailed {
                        handle,
                        exit,
                        cause,
                        ..
                    },
                ..
            } => Some((*handle, *exit, *cause)),
            _ => None,
        })
        .collect()
}

/// Bit-exact sample comparison — `0.5` and `0.0` are exactly
/// representable, so no epsilon is needed or wanted.
fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

/// Dedicated-route happy path (spec AC2/R-V6): `kill_child` →
/// `InstanceFailed` drain → `Destroy` + `Instantiate` + replay →
/// `Restarted` carrying the dying handle as `old`, a fresh
/// generation-bumped `new`, the recipe `bounds` echo and
/// `restored: NotRequested`. The old handle is `Err(InvalidHandle)`
/// stale, `new` reaches the recipe's `Processing` target and takes
/// the caller's endpoint re-attach — `Restarted` carries no
/// endpoint; re-wiring is caller responsibility.
#[test]
fn restarted_dedicated_replays_recipe_and_rewires() {
    let mut host = dedicated_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), true);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });

    let (old, new, echo, restored) = restarted(&events, id).unwrap();
    assert_eq!(old, h);
    assert_ne!(new, old);
    assert_eq!(echo, Some(bounds()));
    assert_eq!(restored, RestoreOutcome::NotRequested);
    // `new` minted on the freed slot — generation bumped, `old`
    // (the dying handle) is stale.
    assert_eq!(host.lifecycle_state(h), None);
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: h }),
        Err(HostError::InvalidHandle)
    );
    assert_eq!(host.lifecycle_state(new), Some(LifecycleState::Processing));
    assert_eq!(sup.current_handle(id), Some(new));
    assert_eq!(host.latency_frames(new), Some(1024));

    // The fresh instance serves the plugin — caller `Restore`/`Save`
    // round-trips — and takes the declared endpoint re-attach.
    host.execute(HostCommand::RestoreState {
        handle: new,
        bytes: vec![0xAB, 0xCD],
    })
    .unwrap();
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: new }),
        Ok(HostReply::State {
            bytes: vec![0xAB, 0xCD]
        })
    );
    let endpoint = host.prepare_endpoint(new, bounds()).unwrap();
    assert_eq!(endpoint.latency_frames(), 1024);
    drop(endpoint);

    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}

/// The watched `InstanceFailed{cause}` consumption keeps the
/// diagnostics half of the contract: `poll`'s drain runs the same
/// `try_recv_notification` that registers the minted-failure
/// record, so even though the supervisor never echoes the
/// notification (the `Restarted` event carries no cause), the
/// record is queryable by handle afterwards — the documented
/// "cause lives on the registry" path (spec R-B8). `new` re-mints
/// on `old`'s freed slot, so `diagnostics_for_handle(new)`
/// resolves the just-failed record keyed by that slot.
#[test]
fn supervised_drain_registers_handle_correlated_record() {
    let mut host = dedicated_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), true);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });
    let (old, new, ..) = restarted(&events, id).unwrap();
    assert_eq!(old, h);

    // The drain minted the failure record keyed by `h`'s slot —
    // `new` shares it, so the record resolves through the fresh
    // handle and carries the consumed notification's cause
    // verbatim (a kill's `{ExitObserved, Transport}` first-
    // observation set — see `l1_proxy_e2e`).
    let record = host
        .diagnostics_for_handle(new)
        .expect("the drained watched failure registered a record");
    assert_eq!(record.handle, Some(h.slot()));
    assert!(matches!(
        record.cause,
        Some(FailureCause::ExitObserved | FailureCause::Transport)
    ));
    assert!(matches!(
        host.diagnostics_query(record.diagnostics_id),
        DiagnosticQuery::Found(found)
            if found.diagnostics_id == record.diagnostics_id
    ));

    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}

/// Bundled recipe targets (spec AC2/R-V6): three watches sharing
/// one child, one kill, one poll — `config: None` stops at
/// `Initialized` (the echoed `bounds` are dead data —
/// `prepare_endpoint` is the defined `Err(Lifecycle)` there),
/// `config: Some` reaches `Activated` (attachable) and
/// `+start_processing` reaches `Processing`.
#[test]
fn restarted_bundled_reaches_each_recipe_state() {
    let mut host = bundled_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let ha = instantiate(&mut host);
    let hb = instantiate(&mut host);
    let hc = instantiate(&mut host);
    let id_a = sup.watch(&mut host, ha, processing_recipe()).unwrap();
    let id_b = sup.watch(&mut host, hb, activated_recipe()).unwrap();
    let id_c = sup.watch(&mut host, hc, minimal_recipe()).unwrap();

    host.kill_child(ha).unwrap();
    for h in [ha, hb, hc] {
        wait_for_failed(&host, h);
    }
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id_a).is_some()
            && restarted(evs, id_b).is_some()
            && restarted(evs, id_c).is_some()
    });

    // One child exit fans all three notifications out atomically —
    // every entry recovered independently on the fresh pooled child.
    let (old_a, new_a, echo_a, restored_a) = restarted(&events, id_a).unwrap();
    let (old_b, new_b, echo_b, restored_b) = restarted(&events, id_b).unwrap();
    let (old_c, new_c, echo_c, restored_c) = restarted(&events, id_c).unwrap();
    assert_eq!((old_a, old_b, old_c), (ha, hb, hc));
    assert_eq!(echo_a, Some(bounds()));
    assert_eq!(echo_b, Some(bounds()));
    assert_eq!(echo_c, Some(bounds()));
    assert_eq!(restored_a, RestoreOutcome::NotRequested);
    assert_eq!(restored_b, RestoreOutcome::NotRequested);
    assert_eq!(restored_c, RestoreOutcome::NotRequested);
    assert_eq!(
        host.lifecycle_state(new_a),
        Some(LifecycleState::Processing)
    );
    assert_eq!(host.lifecycle_state(new_b), Some(LifecycleState::Activated));
    assert_eq!(
        host.lifecycle_state(new_c),
        Some(LifecycleState::Initialized)
    );
    // `config: None` — `Initialized` rejects the echo's
    // `prepare_endpoint` (`dead data` per the recipe contract).
    assert!(matches!(
        host.prepare_endpoint(new_c, bounds()),
        Err(HostError::Lifecycle(_))
    ));
    // `config: Some` — the `Activated` recovery is attachable.
    host.prepare_endpoint(new_b, bounds()).unwrap();

    for (id, new) in [(id_a, new_a), (id_b, new_b), (id_c, new_c)] {
        assert_eq!(sup.unwatch(id), Some(new));
        destroy(&mut host, new);
    }
}

/// `Applied` checkpoint round-trip (spec AC4/R-V3): the caller's
/// pre-failure `SaveState` blob registered via `set_checkpoint`
/// reaches the fresh instance — `SaveState{new}` returns the blob
/// (the worker committed it), and `restored` echoes `Applied`. The
/// `set_checkpoint` wire bound rejects oversized blobs before the
/// wire — they can never kill a child mid-replay.
#[test]
fn checkpoint_applied_roundtrips_state() {
    let mut host = dedicated_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), true);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();

    // Bound check: `len + 17 > WIRE_MAX_PAYLOAD` rejects; exactly
    // fitting still registers (mirrors the unit-level boundary).
    let max_blob = usize::try_from(wire::WIRE_MAX_PAYLOAD).unwrap() - WIRE_COMMAND_ENVELOPE;
    assert_eq!(
        sup.set_checkpoint(id, vec![0; max_blob + 1]),
        Err(SupervisionError::CheckpointTooLarge)
    );
    sup.set_checkpoint(id, vec![0; max_blob]).unwrap();
    assert!(sup.clear_checkpoint(id));

    // Caller captures the live blob and registers it as the
    // checkpoint the restart applies.
    host.execute(HostCommand::RestoreState {
        handle: h,
        bytes: vec![0xAA, 0xBB],
    })
    .unwrap();
    let HostReply::State { bytes } = host.execute(HostCommand::SaveState { handle: h }).unwrap()
    else {
        panic!("unexpected reply");
    };
    sup.set_checkpoint(id, bytes).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });
    let (_, new, _, restored) = restarted(&events, id).unwrap();
    assert_eq!(restored, RestoreOutcome::Applied);
    // The worker actually committed the blob on the fresh instance.
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: new }),
        Ok(HostReply::State {
            bytes: vec![0xAA, 0xBB]
        })
    );

    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}

/// `Err(Unsupported)` restore (spec AC4/R-V3): a stateless plugin's
/// declaration is a skip, not a burn — one attempt recovers with
/// `restored: Skipped`, no `AttemptFailed`/`GaveUp`, and the new
/// instance keeps its seed blob.
#[test]
fn checkpoint_unsupported_skips_without_attempt() {
    let mut host = dedicated_host(&["--state-unsupported"]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    sup.set_checkpoint(id, vec![0xAA, 0xBB]).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });
    // Exactly one event — the skip consumed no attempt budget.
    assert_eq!(events.len(), 1);
    let (_, new, _, restored) = restarted(&events, id).unwrap();
    assert_eq!(restored, RestoreOutcome::Skipped);
    assert_eq!(attempt_faileds(&events, id), []);
    assert_eq!(gave_ups(&events, id), []);
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: new }),
        Ok(HostReply::State {
            bytes: vec![0x11, 0x22, 0x33, 0x44]
        })
    );

    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}

/// `Err(PluginFailed)` restore (spec AC4/R-V3 + AC5): the
/// conservative classification burns the attempt budget —
/// `AttemptFailed` per attempt, exponential-backoff deadlines met
/// only when the injected `Instant` reaches `next_attempt_at`, and
/// `GaveUp { attempts: 3, last_error: Some(PluginFailed) }` at the
/// cap. The minted residue is always cleaned (no lingering), and
/// the `GaveUp` entry stays until `unwatch`.
#[test]
fn checkpoint_pluginfailed_burns_attempts_to_gaveup() {
    let mut host = dedicated_host(&["--fail-restore"]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    sup.set_checkpoint(id, vec![0xAA, 0xBB]).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(events.len(), 1);
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    // The unwind destroyed the `Initialized` minted — no residue.
    assert_eq!(sup.current_handle(id), None);

    // Backoff gate: the injected `now` before `next_attempt_at`
    // issues no attempt.
    assert!(
        sup.poll(&mut host, t0 + Duration::from_millis(249))
            .is_empty()
    );
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(attempt_faileds(&events, id), [(2, HostError::PluginFailed)]);
    assert!(
        sup.poll(&mut host, t0 + Duration::from_millis(749))
            .is_empty()
    );
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(750),
        |evs| !evs.is_empty(),
    );
    assert_eq!(gave_ups(&events, id), [(3, Some(HostError::PluginFailed))]);
    assert_eq!(sup.current_handle(id), None);

    // `GaveUp` is terminal bookkeeping — retained until `unwatch`
    // (still answers `set_checkpoint`, then a residue-free `unwatch`
    // returns `None` and removes it).
    sup.set_checkpoint(id, vec![1]).unwrap();
    assert_eq!(sup.unwatch(id), None);
    assert_eq!(
        sup.set_checkpoint(id, vec![1]),
        Err(SupervisionError::NotWatched)
    );
}

/// `--fail-initialize` (spec AC7/R-V2): `Initialize`'s non-commit
/// `Err(PluginFailed)` leaves a `Created` minted — cleanup
/// `Destroy` is legal and nothing lingers.
#[test]
fn fail_initialize_attempt_cleans_created() {
    let mut host = dedicated_host(&["--fail-initialize"]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    // `Initialize` itself is flagged — the original stays `Created`.
    let h = instantiate(&mut host);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    assert_eq!(sup.current_handle(id), None);

    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(gave_ups(&events, id), [(2, Some(HostError::PluginFailed))]);
    assert_eq!(sup.unwatch(id), None);
}

/// `--fail-start-processing` (spec AC7/R-V2): `StartProcessing`'s
/// non-commit `Err(PluginFailed)` leaves an `Activated` minted —
/// the unwind steps `Deactivate → Destroy` and clears it fully.
#[test]
fn fail_start_processing_attempt_unwinds_activated() {
    let mut host = dedicated_host(&["--fail-start-processing"]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    // The original can reach `Activated` (only `StartProcessing` is
    // the flagged stage — never sent here).
    drive(&mut host, h, Some(config()), false);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    assert_eq!(sup.current_handle(id), None);

    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(gave_ups(&events, id), [(2, Some(HostError::PluginFailed))]);
    assert_eq!(sup.unwatch(id), None);
}

/// `Activate` `Err(Prepare)` (spec AC7/R-V2): an over-the-wire-limit
/// `max_frames` config fails the replay at `Activate` — no worker
/// flag needed — and the `Initialized` minted is destroyed by
/// cleanup.
#[test]
fn activate_over_limit_attempt_fails_prepare() {
    let mut host = dedicated_host(&[]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, None, false);
    let over = ProcessConfig::new(
        SampleRate::new(48_000).unwrap(),
        1,
        wire::WIRE_MAX_FRAMES + 1,
    )
    .unwrap();
    let recipe = RestartRecipe {
        config: Some(over),
        start_processing: false,
        ..minimal_recipe()
    };
    let id = sup.watch(&mut host, h, recipe).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(
        attempt_faileds(&events, id),
        [(1, HostError::Prepare(PrepareError::UnsupportedConfig))]
    );
    assert_eq!(sup.current_handle(id), None);
}

/// Mid-`CreateInstance` death (spec AC7/R-V2 + R-V1): on a
/// `Bundled --crash-after-msgs=4` route the watched-first entry's
/// restart spawns a fresh child (Scan, `CreateInstance`,
/// `Initialize` = 3 msgs — the knob stays armed); the second
/// entry's `Instantiate` reuses that pooled child, whose
/// `CreateInstance` is the 4th received message — `SpawnFailed`
/// *before* a handle mints (`minted: None` — nothing owed, and the
/// never-minted victim emits no notification). The fresh handle's
/// own collateral death is a separate watched entry's recovery.
#[test]
fn mid_create_death_attempt_failed_without_mint() {
    let mut host = bundled_host(&["--crash-after-msgs=4"]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let a = instantiate(&mut host);
    let b = instantiate(&mut host);
    // Watch order fixes attempt order: equal `next_attempt_at`s
    // sort by `SupervisedId`, so `a`'s entry attempts first.
    let id_a = sup.watch(&mut host, a, minimal_recipe()).unwrap();
    let id_b = sup.watch(&mut host, b, minimal_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(a).unwrap();
    wait_for_failed(&host, a);
    wait_for_failed(&host, b);
    // Both fan-out sends must land before the first drain — a torn
    // drain would invert the `(next_attempt_at, id)` attempt order.
    settle_notification();
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| {
        !attempt_faileds(evs, id_b).is_empty()
    });
    assert_eq!(count_restarted(&events, id_a), 1);
    let failures = attempt_faileds(&events, id_b);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0, 1);
    expect_spawn_failed(&host, failures[0].1);
    // No mint, no cleanup owed — and no `Unsupervised`: the
    // never-minted victim emits no notification.
    assert_eq!(sup.current_handle(id_b), None);
    assert_eq!(unsupervised_failures(&events), []);

    // `a`'s fresh handle collateral-dies on the same child —
    // recovered as its own watched entry.
    let events = poll_until_event(&mut sup, &mut host, t0 + Duration::from_millis(1), |evs| {
        count_restarted(evs, id_a) >= 1
    });
    assert_eq!(count_restarted(&events, id_a), 1);

    // Backoff met: `b`'s next attempt lands msg 4 on the child `a`
    // just recovered to — the identical mid-create death.
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !attempt_faileds(evs, id_b).is_empty(),
    );
    let failures = attempt_faileds(&events, id_b);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0, 2);
    expect_spawn_failed(&host, failures[0].1);
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(251),
        |evs| count_restarted(evs, id_a) >= 1,
    );
    assert_eq!(count_restarted(&events, id_a), 1);

    // The bounded crash-loop ends in `GaveUp`.
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(750),
        |evs| !gave_ups(evs, id_b).is_empty(),
    );
    let gave_ups = gave_ups(&events, id_b);
    assert_eq!(gave_ups.len(), 1);
    assert_eq!(gave_ups[0].0, 3);
    expect_spawn_failed(&host, gave_ups[0].1.expect("last_error recorded"));
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(751),
        |evs| count_restarted(evs, id_a) >= 1,
    );
    assert_eq!(count_restarted(&events, id_a), 1);

    assert_eq!(sup.unwatch(id_b), None);
    let current = sup.unwatch(id_a).expect("a recovered live");
    destroy(&mut host, current);
}

/// Crash-loop (spec AC5/R-V4 + AC7 mid-replay death): on
/// `Dedicated --crash-after-msgs=3` every restart attempt's
/// `Initialize` is the child's 3rd received message — `Err(Ipc)`
/// mid-replay with the `Created`/`Failed` minted cleaned by
/// `Destroy`. Bounded retries end at `GaveUp { attempts: 3,
/// last_error: Some(Ipc) }`.
#[test]
fn crash_loop_bounded_gaveup() {
    let mut host = dedicated_host(&["--crash-after-msgs=3"]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    // `Initialize` is the crash message — the original stays
    // `Created`.
    let h = instantiate(&mut host);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::Ipc)]);
    // The minted handle was destroyed by cleanup — no residue.
    assert_eq!(sup.current_handle(id), None);

    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(attempt_faileds(&events, id), [(2, HostError::Ipc)]);
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(750),
        |evs| !evs.is_empty(),
    );
    assert_eq!(gave_ups(&events, id), [(3, Some(HostError::Ipc))]);
    assert_eq!(sup.current_handle(id), None);
    assert_eq!(sup.unwatch(id), None);
}

/// Lingering residue (spec AC7): Dedicated
/// `--fail-start-processing --crash-after-msgs=6` — the restart
/// child's `StartProcessing` (msg 5) fails non-committed, leaving
/// an `Activated` minted; the unwind's `Deactivate` is msg 6 — the
/// crash knob fires on receipt, `Deactivate` gets `Err(Ipc)`, and
/// the live un-destroyable handle becomes `lingering` residue
/// exposed through `current_handle`. The next poll's cleanup retry
/// (`Destroy` from `Failed`, Phase-2 — the drained late-death
/// notification's lingering match is the other defined clear)
/// frees it: no leak, no orphan.
#[test]
fn lingering_residue_retry_destroy_clears() {
    let mut host = dedicated_host(&["--fail-start-processing", "--crash-after-msgs=6"]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), false);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    // The `Activated` minted survived the aborted unwind — tracked,
    // not leaked.
    let lingering = sup.current_handle(id).expect("lingering residue exposed");
    wait_for_failed(&host, lingering);
    settle_notification();
    poll_until_cleared(&mut sup, &mut host, t0, id);
    assert_eq!(sup.current_handle(id), None);
    // Nothing surfaced for the residue — it stayed tracked to the
    // end (no `Unsupervised` leak).
    assert!(
        sup.poll(&mut host, t0 + Duration::from_millis(1))
            .is_empty()
    );
    assert_eq!(sup.unwatch(id), None);
}

/// The external-`Destroy` clear path (spec AC7): the caller frees
/// the lingering handle itself — the next poll's cleanup retry sees
/// `lifecycle_state → None` and clears tracking
/// (`Err(InvalidHandle)`-equivalent).
#[test]
fn lingering_residue_external_destroy_clears() {
    let mut host = dedicated_host(&["--fail-start-processing", "--crash-after-msgs=6"]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), false);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();
    let t0 = Instant::now();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    let lingering = sup.current_handle(id).expect("lingering residue exposed");
    wait_for_failed(&host, lingering);
    // The caller's own `Destroy` — the next poll clears tracking on
    // the freed slot.
    host.execute(HostCommand::Destroy { handle: lingering })
        .unwrap();
    assert_eq!(host.lifecycle_state(lingering), None);
    poll_until_cleared(&mut sup, &mut host, t0, id);
    assert_eq!(sup.unwatch(id), None);
}

/// `unwatch` salvage (spec AC6): `unwatch` returns the live
/// lingering residue so nothing is orphaned — supervision stops and
/// the handle becomes the caller's own bookkeeping.
#[test]
fn lingering_residue_unwatch_returns_handle() {
    let mut host = dedicated_host(&["--fail-start-processing", "--crash-after-msgs=6"]);
    let policy = RestartPolicy {
        max_attempts: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), false);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    let lingering = sup.current_handle(id).expect("lingering residue exposed");
    // `Pending` + residue — `unwatch` hands the live handle back.
    assert_eq!(sup.unwatch(id), Some(lingering));
    assert_eq!(sup.current_handle(id), None);
    wait_for_failed(&host, lingering);
    host.execute(HostCommand::Destroy { handle: lingering })
        .unwrap();
}

/// `GaveUp` re-emit on a lingering late death (spec AC6): with
/// `max_attempts: 1` the same residue lands on a `GaveUp` entry —
/// `current_handle` still exposes it; the residue's own
/// `InstanceFailed` drains as a lingering match and re-emits the
/// identical `GaveUp { attempts, last_error }` (dedupe on
/// `(id, attempts)` against the first emit).
#[test]
fn gaveup_lingering_late_death_reemits() {
    let mut host = dedicated_host(&["--fail-start-processing", "--crash-after-msgs=6"]);
    let policy = RestartPolicy {
        max_attempts: 1,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), false);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| !evs.is_empty());
    // `max_attempts: 1` — the failed attempt turns straight into
    // `GaveUp` (no `AttemptFailed`), the residue staying live.
    assert_eq!(gave_ups(&events, id), [(1, Some(HostError::PluginFailed))]);
    let lingering = sup.current_handle(id).expect("lingering residue exposed");
    wait_for_failed(&host, lingering);
    // The notification's `Pending → Sent` completion must land
    // before the drain — otherwise the Phase-2 retry `Destroy` can
    // preempt the lingering match and the re-emit never surfaces.
    settle_notification();
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        !gave_ups(evs, id).is_empty()
    });
    // Identical `attempts`/`last_error` — `(id, attempts)` dedupe.
    assert_eq!(gave_ups(&events, id), [(1, Some(HostError::PluginFailed))]);
    assert_eq!(sup.current_handle(id), None);
    assert_eq!(sup.unwatch(id), None);
}

/// `Failed` adopt (spec AC9): `watch` on a `Failed` handle destroys
/// the dead slot (a still-queued `InstanceFailed` drops with it —
/// no double recovery), registers `Pending { next_attempt_at: None }`
/// and the next `poll` recovers — identically whether the caller
/// already consumed the notification or it stayed queued.
#[test]
fn adopt_failed_handle_recovers() {
    let mut host = bundled_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let a = instantiate(&mut host);
    let b = instantiate(&mut host);

    host.kill_child(a).unwrap();
    wait_for_failed(&host, a);
    wait_for_failed(&host, b);
    // Both fan-out sends must land before `wait_notification` —
    // otherwise it could pop `b`'s notification first (slot order).
    settle_notification();
    // Variant A: the caller already consumed the notification.
    let notified = wait_notification(&mut host);
    assert!(matches!(
        notified,
        HostNotification::InstanceFailed { handle, .. } if handle == a
    ));
    let id_a = sup.watch(&mut host, a, minimal_recipe()).unwrap();
    // Variant B: the notification is still queued — adopt's
    // `Destroy` frees the slot and it drops with it.
    let id_b = sup.watch(&mut host, b, minimal_recipe()).unwrap();

    // `next_attempt_at: None` — both recover in the same poll, and
    // nothing else is emitted (no notification was needed).
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id_a).is_some() && restarted(evs, id_b).is_some()
    });
    assert_eq!(events.len(), 2);
    let (old_a, new_a, ..) = restarted(&events, id_a).unwrap();
    let (old_b, new_b, ..) = restarted(&events, id_b).unwrap();
    assert_eq!((old_a, old_b), (a, b));
    assert_eq!(
        host.lifecycle_state(new_a),
        Some(LifecycleState::Initialized)
    );
    assert_eq!(
        host.lifecycle_state(new_b),
        Some(LifecycleState::Initialized)
    );

    for (id, new) in [(id_a, new_a), (id_b, new_b)] {
        assert_eq!(sup.unwatch(id), Some(new));
        destroy(&mut host, new);
    }
}

/// `max_attempts = 0` (spec AC5): a `Failed`-handle adopt lands in
/// `Pending` without passing the drain's give-up gate — the next
/// `poll` reports `GaveUp { attempts: 0, last_error: None }` and
/// issues no attempt.
#[test]
fn zero_max_attempts_adopt_gives_up_immediately() {
    let mut host = bundled_host(&[]);
    let policy = RestartPolicy {
        max_attempts: 0,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let h = instantiate(&mut host);

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| !evs.is_empty());
    assert_eq!(events.len(), 1);
    assert_eq!(gave_ups(&events, id), [(0, None)]);
    assert_eq!(sup.current_handle(id), None);
    assert_eq!(sup.unwatch(id), None);
}

/// `stable_window` drain-time reset (spec AC5/R-V4): an episode
/// that ends `failures = 2` must not bleed into the next crash —
/// `healthy_since + stable_window` past, the drain's pre-give-up
/// reset starts a fresh episode (`AttemptFailed { attempt: 1 }`,
/// not `GaveUp { attempts: 3 }`). Deadlines are injected
/// `Instant`s — no real-time dependence.
#[test]
fn stable_window_crash_starts_fresh_episode() {
    let mut host = dedicated_host(&["--fail-restore"]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    let id = sup.watch(&mut host, h, minimal_recipe()).unwrap();
    sup.set_checkpoint(id, vec![0xAA]).unwrap();
    let t0 = Instant::now();

    // First episode: two consumed attempts.
    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| !evs.is_empty());
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    let events = poll_until_event(
        &mut sup,
        &mut host,
        t0 + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(attempt_faileds(&events, id), [(2, HostError::PluginFailed)]);

    // Recover with `failures = 2` on the clock — `healthy_since` is
    // this poll's `now` (t0 + 750 ms).
    assert!(sup.clear_checkpoint(id));
    let recovered_at = t0 + Duration::from_millis(750);
    let events = poll_until_event(&mut sup, &mut host, recovered_at, |evs| {
        restarted(evs, id).is_some()
    });
    let (_, new, ..) = restarted(&events, id).unwrap();

    // 61 s later (`stable_window` is 60 s), the next crash opens a
    // fresh episode — the drain reset the stale `failures` first.
    sup.set_checkpoint(id, vec![0xBB]).unwrap();
    host.kill_child(new).unwrap();
    wait_for_failed(&host, new);
    // The notification must be queued before the `later` poll —
    // otherwise Phase-2's `Live` sweep resets `failures` first and
    // the pinned Phase-1 drain reset is never exercised.
    settle_notification();
    let later = recovered_at + Duration::from_secs(61);
    let events = poll_until_event(&mut sup, &mut host, later, |evs| !evs.is_empty());
    assert_eq!(events.len(), 1);
    // `attempt: 1` — the regression pin: without the drain-time
    // reset this is `GaveUp { attempts: 3 }` on stale failures.
    assert_eq!(attempt_faileds(&events, id), [(1, HostError::PluginFailed)]);
    assert_eq!(gave_ups(&events, id), []);

    // The fresh episode retries normally — clear the checkpoint and
    // the next attempt succeeds.
    let events = poll_until_event(
        &mut sup,
        &mut host,
        later + Duration::from_millis(250),
        |evs| !evs.is_empty(),
    );
    assert_eq!(attempt_faileds(&events, id), [(2, HostError::PluginFailed)]);
    assert!(sup.clear_checkpoint(id));
    let events = poll_until_event(
        &mut sup,
        &mut host,
        later + Duration::from_millis(750),
        |evs| restarted(evs, id).is_some(),
    );
    let (_, new, ..) = restarted(&events, id).unwrap();
    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}

/// Bundle burst (spec AC3/R-V1): one child's death fans one
/// `InstanceFailed` out to every co-resident instance — four
/// watched plus one unwatched. `max_attempts_per_poll: 2` bounds
/// the spawn/reply-wait concentration: the first poll drains all
/// five notifications, attempts two and defers two silently
/// (`Pending` emits nothing — `current_handle → None` is the
/// observable); the unwatched pass-through is verbatim. The pool
/// converges recovery onto one fresh child — the second kill
/// repeats the cycle.
#[test]
fn bundle_burst_cap_defers_and_unsupervised() {
    let mut host = bundled_host(&[]);
    let policy = RestartPolicy {
        max_attempts_per_poll: 2,
        ..RestartPolicy::default()
    };
    let mut sup = Supervisor::new(policy).unwrap();
    let watched: Vec<PluginHandle> = (0..4).map(|_| instantiate(&mut host)).collect();
    let ids: Vec<SupervisedId> = watched
        .iter()
        .map(|&h| sup.watch(&mut host, h, minimal_recipe()).unwrap())
        .collect();
    let unwatched = instantiate(&mut host);
    let t0 = Instant::now();

    host.kill_child(watched[0]).unwrap();
    for &h in watched.iter().chain(std::iter::once(&unwatched)) {
        wait_for_failed(&host, h);
    }
    // All five fan-out sends must land before the first drain — a
    // torn drain would leave entries `Live` or drop `Unsupervised`.
    settle_notification();
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| {
        count_restarted(evs, ids[0]) + count_restarted(evs, ids[1]) >= 2
    });
    // The unwatched handle passes through with the real exit and
    // the first-observed cause — the pass-through is verbatim.
    let fails = unsupervised_failures(&events);
    assert_eq!(fails.len(), 1);
    assert_eq!(fails[0].0, unwatched);
    assert!(fails[0].1.code.is_some() || fails[0].1.signal.is_some());
    assert!(matches!(
        fails[0].2,
        FailureCause::ExitObserved | FailureCause::Transport
    ));
    // Cap met: two `Restarted`, two deferred — the deferral is
    // silent, so `current_handle` is the observable.
    assert_eq!(count_restarted(&events, ids[0]), 1);
    assert_eq!(count_restarted(&events, ids[1]), 1);
    assert!(sup.current_handle(ids[0]).is_some());
    assert!(sup.current_handle(ids[1]).is_some());
    assert_eq!(sup.current_handle(ids[2]), None);
    assert_eq!(sup.current_handle(ids[3]), None);

    // The carried-over `Pending` entries run next poll — the fresh
    // pooled child serves every replay.
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| {
        count_restarted(evs, ids[2]) + count_restarted(evs, ids[3]) >= 2
    });
    assert_eq!(count_restarted(&events, ids[2]), 1);
    assert_eq!(count_restarted(&events, ids[3]), 1);
    for &id in &ids {
        assert!(sup.current_handle(id).is_some());
    }
    // The unwatched instance stays `Failed` — the caller's manual
    // recovery is preserved.
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: unwatched }),
        Err(HostError::Failed)
    );

    // One fresh child serves all four — a second kill re-fans the
    // same burst cycle.
    let current: Vec<PluginHandle> = ids
        .iter()
        .map(|&id| sup.current_handle(id).unwrap())
        .collect();
    host.kill_child(current[0]).unwrap();
    for &h in &current {
        wait_for_failed(&host, h);
    }
    settle_notification();
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| {
        count_restarted(evs, ids[0]) + count_restarted(evs, ids[1]) >= 2
    });
    assert_eq!(count_restarted(&events, ids[0]), 1);
    assert_eq!(count_restarted(&events, ids[1]), 1);
    assert_eq!(sup.current_handle(ids[2]), None);
    assert_eq!(sup.current_handle(ids[3]), None);
    let events = poll_until_event(&mut sup, &mut host, t0, |evs| {
        count_restarted(evs, ids[2]) + count_restarted(evs, ids[3]) >= 2
    });
    assert_eq!(count_restarted(&events, ids[2]), 1);
    assert_eq!(count_restarted(&events, ids[3]), 1);

    // Caller-side manual teardown for the unwatched `Failed`
    // instance plus the watched currents.
    host.execute(HostCommand::Destroy { handle: unwatched })
        .unwrap();
    for &id in &ids {
        let current = sup.unwatch(id).expect("recovered live");
        destroy(&mut host, current);
    }
}

/// Host binding (spec AC1/host contract): one `Supervisor` serves
/// one `PluginHost` — `watch`/`set_recipe`/`poll` on a foreign host
/// are the defined `WrongHost` rejection, and `poll` drains nothing
/// on the foreign queue (the bound host's notifications belong to
/// its own drainer).
#[test]
fn wrong_host_rejected_without_drain() {
    let mut host_a = bundled_host(&[]);
    let mut host_b = bundled_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let a = instantiate(&mut host_a);
    let id = sup.watch(&mut host_a, a, minimal_recipe()).unwrap();
    let b = instantiate(&mut host_b);

    assert_eq!(
        sup.watch(&mut host_b, b, minimal_recipe()),
        Err(SupervisionError::WrongHost)
    );
    assert_eq!(
        sup.set_recipe(&mut host_b, id, minimal_recipe()),
        Err(SupervisionError::WrongHost)
    );
    host_b.kill_child(b).unwrap();
    wait_for_failed(&host_b, b);
    // `poll` on the foreign host reports `WrongHost` and touches
    // nothing — no drain, no reset, no attempt.
    assert!(matches!(
        sup.poll(&mut host_b, Instant::now()).as_slice(),
        [SupervisionEvent::WrongHost]
    ));
    let notified = wait_notification(&mut host_b);
    assert!(matches!(
        notified,
        HostNotification::InstanceFailed { handle, .. } if handle == b
    ));

    // The bound host still works.
    host_a.kill_child(a).unwrap();
    wait_for_failed(&host_a, a);
    let events = poll_until_event(&mut sup, &mut host_a, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });
    let (_, new, ..) = restarted(&events, id).unwrap();
    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host_a, new);
}

/// Opt-in non-regression (spec AC1/R-V5/AC4): a plain `PluginHost`
/// — no supervisor — keeps the base contract: `kill_child` surfaces
/// `InstanceFailed`, the instance stays `Failed` until the caller
/// destroys it, and recovery is manual. No `Restarted`-equivalent
/// exists outside the supervisor.
#[test]
fn plain_host_reports_failed_no_autorestart() {
    let mut host = bundled_host(&[]);
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), true);

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let notified = wait_notification(&mut host);
    assert!(matches!(
        notified,
        HostNotification::InstanceFailed { handle, .. } if handle == h
    ));
    // Terminal `Failed` until the caller decides — no autonomous
    // recovery.
    assert_eq!(host.lifecycle_state(h), Some(LifecycleState::Failed));
    assert_eq!(
        host.execute(HostCommand::SaveState { handle: h }),
        Err(HostError::Failed)
    );
    host.execute(HostCommand::Destroy { handle: h }).unwrap();
    // The caller's manual recovery is the same series the
    // supervisor replays.
    let h2 = instantiate(&mut host);
    drive(&mut host, h2, Some(config()), true);
    destroy(&mut host, h2);
}

/// `max_attempts_per_poll == 0` is the inert policy —
/// `Supervisor::new` rejects it `Err(InertPolicy)` rather than
/// silently clamping.
#[test]
fn inert_policy_rejected() {
    let policy = RestartPolicy {
        max_attempts_per_poll: 0,
        ..RestartPolicy::default()
    };
    assert_eq!(
        Supervisor::new(policy).err(),
        Some(SupervisionError::InertPolicy)
    );
}

/// RT lane (Issue #22 shape): the recovered instance's endpoint —
/// caller re-attached after `Restarted` per the recipe's `bounds`
/// echo — runs the `process_audio` block path allocation-free.
/// Fixture, supervisor, attach and warm-up all sit outside the
/// `assert_no_alloc` window; the supervisor itself never touches
/// the RT path (non-RT by contract).
#[test]
fn rt_l1_supervised_process_no_alloc() {
    let mut host = dedicated_host(&[]);
    let mut sup = Supervisor::new(RestartPolicy::default()).unwrap();
    let h = instantiate(&mut host);
    drive(&mut host, h, Some(config()), true);
    let id = sup.watch(&mut host, h, processing_recipe()).unwrap();

    host.kill_child(h).unwrap();
    wait_for_failed(&host, h);
    let events = poll_until_event(&mut sup, &mut host, Instant::now(), |evs| {
        restarted(evs, id).is_some()
    });
    let (_, new, echo, _) = restarted(&events, id).unwrap();
    // Caller-side endpoint re-attach — `Restarted` carries no
    // endpoint, so this is the documented re-wire step.
    let mut endpoint = host.prepare_endpoint(new, echo.unwrap()).unwrap();

    let ctx = RtContext::mint();
    let mut slab = [0.0_f32; 16];
    // Warm-up outside the window: first-touch staging pages, the
    // driver's `AddInstance` attach and the lookahead horizon.
    for _ in 0..8 {
        slab.fill(1.0);
        endpoint
            .process_audio(
                &mut AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
                EventList::empty(8),
                None,
                &ctx,
            )
            .unwrap();
    }
    assert_no_alloc::assert_no_alloc(|| {
        for _ in 0..32 {
            slab.fill(1.0);
            let result = endpoint.process_audio(
                &mut AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
                EventList::empty(8),
                None,
                &ctx,
            );
            assert_eq!(result, Ok(ProcessResult::CONTINUE));
            // Defined outputs only — silence or the worker's `0.5 ×`
            // gain (underrun pass-through included).
            assert!(
                slab.iter().all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)),
                "undefined sample on the supervised leg"
            );
        }
    });

    assert_eq!(sup.unwatch(id), Some(new));
    destroy(&mut host, new);
}
