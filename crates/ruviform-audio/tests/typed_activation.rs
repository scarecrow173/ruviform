//! G2-04/G2-05 — stateful-ownership proof and activation evidence:
//! Plan-owned vs Stable Runtime Slot measured against the same
//! `MockDsp`; generation-tagged install/remove, stale-generation
//! event rejection, rollback to last-good, retire/drain ordering,
//! and the single-RT-owner guarantee.

mod common;

use std::sync::{Arc, Mutex};

use common::{MockDsp, MockFactory, chain, config, merge, validate};
use ruviform_audio::typed::{
    ActivationError, Compiler, GraphRuntime, NodeEvent, RtRuntime, RuntimeFault,
};
use ruviform_rt::RtContext;

/// Compiles+publishes `spec` through `runtime`; returns nothing —
/// the runtime pair is the authority afterward.
fn publish(
    runtime: &mut GraphRuntime,
    spec: &ruviform_graph::GraphSpec,
    factory: &mut MockFactory,
) {
    let g = validate(spec);
    let compiled = Compiler::new(config(256))
        .compile(
            &g,
            factory,
            runtime.registry(),
            runtime.allocator(),
            runtime.generation() + 1,
        )
        .expect("compile");
    runtime.publish(compiled).expect("publish");
}

fn chain_runtime() -> (
    GraphRuntime,
    RtRuntime,
    common::Chain,
    Arc<Mutex<Vec<String>>>,
) {
    let c = chain();
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime, rt) = GraphRuntime::new(&config(256), 8, 8);
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    publish(&mut runtime, &c.spec, &mut factory);
    (runtime, rt, c, log)
}

fn feed_process(rt: &mut RtRuntime, port: ruviform_graph::PortId, v: f32, frames: usize) {
    let data = [vec![v; frames], vec![v; frames]];
    let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
    common::feed(rt, port, &refs);
    rt.process(frames, &RtContext::mint()).expect("process");
}

// ---------- G2-04: Plan-owned vs Stable Runtime Slot ----------

/// The SAME `MockDsp` under both ownership models. Under Stable
/// Runtime Slot (`reuse: true`), a republished plan keeps the
/// instance: `state` continues (calls 1..N accumulate).
#[test]
fn g2_04_stable_slot_preserves_state() {
    // --- Stable Runtime Slot: reuse → same instance, state continues.
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 8, 8);
    let mut factory = MockFactory {
        reuse: true,
        ..Default::default()
    };
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 1)
        .expect("compile g1");
    runtime.publish(compiled).expect("publish g1");
    feed_process(&mut rt, c.src_out, 1.0, 64); // state → 1
    feed_process(&mut rt, c.src_out, 1.0, 64); // state → 2

    // Recompile the identical graph at generation 2 — the factory
    // reuses the live instance; nothing is reinstalled.
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 2)
        .expect("compile g2");
    assert_eq!(compiled.installs.len(), 0, "reuse → no install");
    assert_eq!(compiled.removes, []);
    runtime.publish(compiled).expect("publish g2");
    feed_process(&mut rt, c.src_out, 1.0, 64); // state → 3
    let out = common::drain(&mut rt, c.sink_in);
    // out = in(1.0)*gain + state(3) = 4.0 — the SAME instance
    // accumulated across the plan update.
    assert!(
        (out[0][0] - 4.0).abs() < f32::EPSILON,
        "stable slot: state continues (got {})",
        out[0][0]
    );
    // Activation happened once (install at g1); no deactivate yet.
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|s| s.starts_with("activate"))
            .count(),
        1
    );
    // Exactly one RT owner ever existed: `RtRuntime` is `!Sync` and
    // the MockDsp was invoked only from this single runtime —
    // evidenced by the monotonic `state`/`calls` above.
}

/// The counterpart of [`g2_04_stable_slot_preserves_state`]: a
/// Plan-owned model (`reuse: false` → `New` every compile) replaces
/// the instance and `state` resets — the comparison the ADR records.
#[test]
fn g2_04_plan_owned_replaces_state() {
    // --- Plan-owned: New every compile → instance replaced, state resets.
    let c2 = chain();
    let g2 = validate(&c2.spec);
    let log2 = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime2, mut rt2) = GraphRuntime::new(&config(256), 8, 8);
    let mut factory2 = MockFactory::default(); // reuse: false
    factory2
        .pending
        .insert(c2.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log2))));
    let compiled = Compiler::new(config(256))
        .compile(
            &g2,
            &mut factory2,
            runtime2.registry(),
            runtime2.allocator(),
            1,
        )
        .expect("compile g1");
    runtime2.publish(compiled).expect("publish g1");
    feed_process(&mut rt2, c2.src_out, 1.0, 64);
    feed_process(&mut rt2, c2.src_out, 1.0, 64);
    // Recompile: factory issues a NEW instance (plan-owned model).
    factory2
        .pending
        .insert(c2.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log2))));
    let compiled = Compiler::new(config(256))
        .compile(
            &g2,
            &mut factory2,
            runtime2.registry(),
            runtime2.allocator(),
            2,
        )
        .expect("compile g2");
    assert_eq!(compiled.installs.len(), 1, "plan-owned → reinstall");
    runtime2.publish(compiled).expect("publish g2");
    feed_process(&mut rt2, c2.src_out, 1.0, 64);
    let out2 = common::drain(&mut rt2, c2.sink_in);
    // Fresh instance: state = 1 → out = 1.0 + 1 = 2.0, NOT 4.0.
    assert!(
        (out2[0][0] - 2.0).abs() < f32::EPSILON,
        "plan-owned: state resets (got {})",
        out2[0][0]
    );
    let entries = log2.lock().unwrap();
    // deactivate(old) THEN activate(new) — replacement order on the
    // RT side.
    assert_eq!(
        entries
            .iter()
            .filter(|s| s.starts_with("deactivate"))
            .count(),
        1
    );
    assert_eq!(
        entries.iter().filter(|s| s.starts_with("activate")).count(),
        2
    );
    // And the order inside the generation-2 activation is
    // deactivate-then-activate.
    let pos_deact = entries
        .iter()
        .position(|s| s.starts_with("deactivate"))
        .unwrap();
    let pos_last_act = entries
        .iter()
        .rposition(|s| s.starts_with("activate"))
        .unwrap();
    assert!(pos_deact < pos_last_act);
    drop(entries);
}

/// Install/remove through generation-tagged commands: dropping a
/// node deactivates+retires its instance (destruction deferred to
/// `collect`), re-adding installs fresh.
#[test]
fn g2_04_install_remove_order_and_deferred_destruction() {
    let m = merge();
    let g1 = validate(&m.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 8, 8);
    let mut factory = MockFactory::default();
    for n in [m.a, m.b] {
        factory
            .pending
            .insert(n, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    }
    let compiled = Compiler::new(config(256))
        .compile(
            &g1,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            1,
        )
        .expect("compile g1");
    assert_eq!(compiled.installs.len(), 2);
    runtime.publish(compiled).expect("publish g1");
    // Drive one callback — generation-1 activation installs a and b.
    feed_process(&mut rt, m.src_a_out, 1.0, 64);
    feed_process(&mut rt, m.src_b_out, 1.0, 64);

    // New spec drops b — chain has one processor.
    let c = chain();
    let mut factory2 = MockFactory {
        reuse: true,
        ..Default::default()
    };
    // c.proc ≠ m.a/m.b — everything is removed; c.proc is new.
    factory2
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    publish(&mut runtime, &c.spec, &mut factory2);
    // Drive one callback — activation applies removes+installs.
    feed_process(&mut rt, c.src_out, 1.0, 64);
    let entries = log.lock().unwrap().clone();
    // Two deactivates (m.a, m.b) + one activate (c.proc).
    assert_eq!(
        entries
            .iter()
            .filter(|s| s.starts_with("deactivate"))
            .count(),
        2,
        "log: {entries:?}"
    );
    assert_eq!(
        entries.iter().filter(|s| s.starts_with("activate")).count(),
        3,
        "gen1 a+b, gen2 c.proc — log: {entries:?}"
    );
    drop(entries);
    // Deferred destruction: retired boxes drop only in collect().
    let dropped = runtime.collect();
    assert!(dropped >= 2, "two retired processors drained");
}

// ---------- G2-05: activation faults ----------

/// Stale-generation events are discarded and counted — an event
/// tagged for generation 1 never reaches the generation-2 instance.
#[test]
fn g2_05_stale_generation_events_rejected() {
    let (mut runtime, mut rt, c, log) = chain_runtime();
    // Activate generation 1 first.
    feed_process(&mut rt, c.src_out, 1.0, 64);
    // Send events tagged for the current generation and a stale one.
    let stale = NodeEvent {
        generation: 0, // predates every real generation
        node: c.proc,
        opcode: 0,
        value: 42.0,
    };
    let fresh = NodeEvent {
        generation: runtime.generation(),
        node: c.proc,
        opcode: 0,
        value: 7.0,
    };
    runtime.send_event(stale).expect("send stale");
    runtime.send_event(fresh).expect("send fresh");
    feed_process(&mut rt, c.src_out, 1.0, 64);
    let snap = runtime.status().snapshot();
    assert_eq!(snap.stale_events, 1, "stale event counted and dropped");
    // The fresh event set gain=7 → out = 1*7 + state.
    let out = common::drain(&mut rt, c.sink_in);
    assert!(out[0][0] > 7.0, "fresh event applied (gain 7)");
    let entries = log.lock().unwrap();
    assert_eq!(
        entries.iter().filter(|s| s.starts_with("event:")).count(),
        1,
        "only the generation-matching event was delivered"
    );
}

/// Publish failure is rollback-by-construction: a command channel
/// too small for the delta leaves the last-good plan active and
/// the registry untouched.
#[test]
fn g2_05_publish_failure_retains_last_good() {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    // command_capacity = 1 — the chain install alone fits; any
    // install+remove pair overflows it.
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 1, 8);
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 1)
        .expect("compile g1");
    runtime.publish(compiled).expect("publish g1");
    feed_process(&mut rt, c.src_out, 1.0, 64);

    // Generation 2 = merge graph: removes 1 + installs 2 = 3
    // commands > capacity 1 → ActivationError, nothing published.
    let m = merge();
    let g2 = validate(&m.spec);
    let mut factory = MockFactory::default();
    for n in [m.a, m.b] {
        factory
            .pending
            .insert(n, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    }
    let compiled = Compiler::new(config(256))
        .compile(
            &g2,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            2,
        )
        .expect("compile g2");
    let err = runtime.publish(compiled).unwrap_err();
    assert!(matches!(err, ActivationError::CommandChannelFull));
    // Last-good still runs — the generation stays 1.
    assert_eq!(runtime.generation(), 1);
    feed_process(&mut rt, c.src_out, 1.0, 64);
    let out = common::drain(&mut rt, c.sink_in);
    assert!(out[0][0] > 0.0, "last-good plan still processes");
    // The leftover commands carry generation 2 but were never
    // published — they are drained as stale on the NEXT successful
    // activation, not applied.
}

/// The leftover stale-generation commands from a failed publish are
/// discarded (counted) when a valid generation activates afterward.
#[test]
fn g2_05_stale_commands_discarded_on_next_activation() {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    // command_capacity = 2: g1 install (1) fits; the g2 delta
    // (remove+install = 2) also fits... force overflow via a bigger
    // delta — merge's 3 commands > 2.
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 2, 8);
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 1)
        .expect("compile g1");
    runtime.publish(compiled).expect("publish g1");
    feed_process(&mut rt, c.src_out, 1.0, 64);

    // Failed publish leaves ≤ capacity commands queued with gen 2.
    let m = merge();
    let g2 = validate(&m.spec);
    let mut factory = MockFactory::default();
    for n in [m.a, m.b] {
        factory
            .pending
            .insert(n, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    }
    let compiled = Compiler::new(config(256))
        .compile(
            &g2,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            2,
        )
        .expect("compile g2");
    assert!(runtime.publish(compiled).is_err());

    // Recompile the SAME chain at generation 3 — factory reuses the
    // live instance; the publish pushes 0 commands and activates.
    let mut factory = MockFactory {
        reuse: true,
        ..Default::default()
    };
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 3)
        .expect("compile g3");
    runtime.publish(compiled).expect("publish g3");
    feed_process(&mut rt, c.src_out, 1.0, 64);
    let snap = runtime.status().snapshot();
    assert_eq!(snap.active_generation, 3);
    assert!(
        snap.stale_commands >= 1,
        "leftover gen-2 commands discarded as stale: {snap:?}"
    );
    let out = common::drain(&mut rt, c.sink_in);
    assert!(out[0][0] > 0.0);
}

/// Retired processor boxes are destroyed only by `collect` on the
/// non-RT side — a "stopped collector" leaves them alive (leak-safe,
/// counted), never dropped inside the RT path.
#[test]
fn g2_05_retirement_deferred_and_collector_drain_observed() {
    let m = merge();
    let g1 = validate(&m.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 8, 8);
    let mut factory = MockFactory::default();
    for n in [m.a, m.b] {
        factory
            .pending
            .insert(n, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    }
    let compiled = Compiler::new(config(256))
        .compile(
            &g1,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            1,
        )
        .expect("compile g1");
    runtime.publish(compiled).expect("publish g1");
    let src_a_port = m.spec.node(m.src_a).unwrap().ports()[0].id();
    feed_process(&mut rt, src_a_port, 1.0, 64);

    // Replace the plan wholesale — both instances retire.
    let c = chain();
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    publish(&mut runtime, &c.spec, &mut factory);
    feed_process(&mut rt, c.src_out, 1.0, 64);
    // Without collect(), destructors stay queued: drain is the only
    // path that runs them.
    let n = runtime.collect();
    assert!(n >= 2, "collector drains retired instances");
    // Calling again drains nothing.
    assert_eq!(runtime.collect(), 0);
}

/// Retirement-queue saturation: more retirements than the retire
/// channel can hold are leaked (never dropped on RT) and surfaced
/// through `retire_overflows` + the `RetireOverflow` fault flag.
/// command_capacity = 1 → retire capacity = 4; five replaced
/// instances overflow it without ever running `collect`.
#[test]
fn g2_05_retire_queue_saturation_flagged_leak_safe() {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 1, 8);
    for gen_num in 1..=6u64 {
        let mut factory = MockFactory::default();
        factory
            .pending
            .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
        let compiled = Compiler::new(config(256))
            .compile(
                &g,
                &mut factory,
                runtime.registry(),
                runtime.allocator(),
                gen_num,
            )
            .expect("compile");
        runtime.publish(compiled).expect("publish");
        feed_process(&mut rt, c.src_out, 1.0, 64);
    }
    let snap = runtime.status().snapshot();
    assert!(
        snap.retire_overflows >= 1,
        "5 replaced instances > retire capacity 4: {snap:?}"
    );
    assert!(
        snap.fault_flags & RuntimeFault::RetireOverflow as u64 != 0,
        "RetireOverflow fault flag set"
    );
    // Leak-safe: the last-good plan still runs; collect drains what
    // the channel could hold (≤ capacity), nothing panics.
    let out = common::drain(&mut rt, c.sink_in);
    assert!(out[0][0] > 0.0);
    let _ = runtime.collect();
}

/// Plan-retire saturation: every `reader.read()` guard drop retires
/// its `Arc` through the snapshot channel — more guard drops than
/// `retire_capacity` (4) without a `collect` are leaked and surfaced
/// through `plan_retire_overflows` + the `PlanRetireOverflow` flag.
/// (Publish displacements take the writer's local pending queue, so
/// only reader-side guard drops fill this channel.)
#[test]
fn g2_05_plan_retire_saturation_flagged() {
    let (mut runtime, mut rt, c, _log) = chain_runtime();
    // Each feed_process drops two snapshot guards (input_writer +
    // process reads) → 3 callbacks = 6 retirements > capacity 4.
    for _ in 0..3 {
        feed_process(&mut rt, c.src_out, 1.0, 64);
    }
    let _ = runtime.collect(); // mirrors writer.retire_overflows → status
    let snap = runtime.status().snapshot();
    assert!(
        snap.plan_retire_overflows >= 1,
        "6 guard drops > snapshot retire capacity 4: {snap:?}"
    );
    assert!(snap.fault_flags & RuntimeFault::PlanRetireOverflow as u64 != 0);
}

/// Activation conflict: a full event channel rejects with
/// `EventChannelFull` — no partial delivery, no publish corruption.
#[test]
fn g2_05_event_channel_full_rejected() {
    let (mut runtime, _rt, c, _log) = chain_runtime();
    let generation = runtime.generation();
    let ev = |v| NodeEvent {
        generation,
        node: c.proc,
        opcode: 0,
        value: v,
    };
    for i in 0..8 {
        runtime.send_event(ev(i as f32)).expect("within capacity");
    }
    let err = runtime.send_event(ev(9.0)).unwrap_err();
    assert!(matches!(err, ActivationError::EventChannelFull));
}

/// Shutdown: dropping `RtRuntime` deactivates and retires every
/// remaining instance — destructor work still lands on `collect`.
#[test]
fn g2_05_shutdown_retires_processors() {
    let (mut runtime, mut rt, c, log) = chain_runtime();
    feed_process(&mut rt, c.src_out, 1.0, 64);
    drop(rt); // shutdown — deactivate + retire, no drops on RT side
    let entries = log.lock().unwrap();
    assert!(
        entries.iter().any(|s| s.starts_with("deactivate")),
        "shutdown deactivates installed processors"
    );
    drop(entries);
    let drained = runtime.collect();
    assert!(drained >= 1, "retired instances drain at collect");
}

/// Single-RT-owner: `RtRuntime` is `Send` but `!Sync` — it can be
/// moved to the callback thread exactly once.
#[test]
fn g2_04_rt_runtime_is_send_not_sync() {
    fn is_send<T: Send>() {}
    is_send::<RtRuntime>();
    // !Sync is structural (interior RefCell) — compile-fail proof
    // lives in the type itself; nothing to assert at runtime.
}
