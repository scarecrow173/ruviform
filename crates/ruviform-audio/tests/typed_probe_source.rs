//! Issue #293 (OBS-11) — concrete `ProbeSource` adapter tests.
//!
//! `ProbeSourceAdapter` wraps the non-RT halves of `pair()` and maps
//! broker/collector state into `ruviform_observability::debug`'s
//! `ProbeSource` contract. These tests cover the drain→snapshot
//! mapping, status/counter accounting, clock-mark semantics, dynamic
//! rebind edges, and registration through `DebugInspector` /
//! `ObservationRunner::debug_mut()`.
mod common;

use common::*;

use ruviform_audio::typed::probe::{
    CaptureConfig, ProbeConfig, ProbeEvent, ProbeId, ProbeMode, ProbePoint, ProbeSourceAdapter,
    pair,
};
use ruviform_audio::typed::{Compiler, GraphProcessor, GraphRuntime, NodeIo, RtRuntime};
use ruviform_graph::{Direction, GraphBuilder, GraphSpec, NodeId, NodeKind, PortId, PortRole};
use ruviform_observability::collector::CollectorConfig;
use ruviform_observability::debug::{
    DebugConfig, DebugInspector, InspectionDetail, InspectionInputs, InspectionRequest,
    InspectionSection, InspectionTarget, ProbeSource, SectionStatus,
};
use ruviform_observability::integration::{DebugSettings, ObservabilityConfig, ObservationRunner};
use ruviform_observability::source::Confidentiality;
use ruviform_rt::RtContext;
use static_assertions::assert_impl_all;
use std::time::{Duration, Instant};

const FRAMES: usize = 64;

// The adapter is the host-facing registration object — it must be
// `Send` (the `ProbeSource` bound) and `Sync` so one clone can be
// registered while the original keeps serving control calls.
assert_impl_all!(ProbeSourceAdapter: Send, Sync);

// ---- fixtures (mirror tests/typed_probe.rs) ----

struct ProbeChain {
    spec: GraphSpec,
    src_out: PortId,
    proc_in: PortId,
    proc_out: PortId,
    sink_in: PortId,
    proc: NodeId,
    sink: NodeId,
}

fn probe_chain() -> ProbeChain {
    probe_chain_inner(true)
}

/// Same node/port issuance order as [`probe_chain`] — IDs are
/// per-builder sequential, so both produce identical
/// `NodeId`/`PortId`s — but with `connect_proc` controlling whether
/// the processor participates in the graph. `false` leaves `proc`
/// isolated: its ports keep their declarations yet resolve to no
/// backing slot (`Unavailable`), which is exactly the rebind-drop
/// edge the adapter must surface.
fn probe_chain_inner(connect_proc: bool) -> ProbeChain {
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&node(
        NodeKind::Source,
        vec![port_probe(Direction::Output, PortRole::Main, stereo())],
    ));
    let (proc, pp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port_probe(Direction::Input, PortRole::Main, stereo()),
            port_probe(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (sink, kp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    if connect_proc {
        b.connect(&conn(pref(src, sp[0]), pref(proc, pp[0])));
        b.connect(&conn(pref(proc, pp[1]), pref(sink, kp[0])));
    } else {
        b.connect(&conn(pref(src, sp[0]), pref(sink, kp[0])));
    }
    ProbeChain {
        spec: b.build(),
        src_out: sp[0],
        proc_in: pp[0],
        proc_out: pp[1],
        sink_in: kp[0],
        proc,
        sink,
    }
}

fn gain2() -> Box<dyn GraphProcessor> {
    Box::new(|io: &mut NodeIo<'_>, _ctx: &RtContext| {
        let input = io.input(0);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for c in 0..out.channels() {
                let Some(plane) = out.channel_mut(c) else {
                    continue;
                };
                if let Some(src) = input.as_ref().and_then(|v| v.channel(c)) {
                    for (d, s) in plane.iter_mut().zip(src.iter()) {
                        *d = s * 2.0;
                    }
                } else {
                    plane.fill(0.0);
                }
            }
        }
    })
}

struct Rig {
    grt: GraphRuntime,
    rt: RtRuntime,
    probe: ProbeSourceAdapter,
}

fn rig(spec: &GraphSpec, probe_config: &ProbeConfig, factory: &mut MockFactory) -> Rig {
    let cfg = config(FRAMES);
    let compiler = Compiler::new(cfg.clone());
    let (mut grt, mut rt) = GraphRuntime::new(&cfg, 8, 8);
    let plan_out = compiler
        .compile(
            &validate(spec),
            factory,
            grt.registry(),
            grt.allocator(),
            grt.generation() + 1,
        )
        .expect("compile");
    grt.publish(plan_out).expect("publish");
    let parts = pair(probe_config);
    rt.set_probe_deck(parts.deck);
    Rig {
        grt,
        rt,
        probe: ProbeSourceAdapter::new(parts.broker, parts.collector),
    }
}

fn ctx() -> RtContext {
    RtContext::mint()
}

#[expect(clippy::cast_precision_loss)]
fn feed_ramp(rt: &mut RtRuntime, port: PortId, offset: f32) {
    let l: Vec<f32> = (0..FRAMES)
        .map(|i| (i as f32 + offset) / FRAMES as f32)
        .collect();
    let r: Vec<f32> = (0..FRAMES)
        .map(|i| ((i as f32) * 0.5 + offset) / FRAMES as f32)
        .collect();
    feed(rt, port, &[&l, &r]);
}

fn block(rig: &mut Rig, c: &ProbeChain) {
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
}

fn republish(rig: &mut Rig, spec: &GraphSpec, factory: &mut MockFactory) {
    let compiler = Compiler::new(config(FRAMES));
    let plan_out = compiler
        .compile(
            &validate(spec),
            factory,
            rig.grt.registry(),
            rig.grt.allocator(),
            rig.grt.generation() + 1,
        )
        .unwrap();
    rig.grt.publish(plan_out).unwrap();
}

fn install_monitor(rig: &Rig, c: &ProbeChain, port: PortId) -> ProbeId {
    rig.probe
        .install(
            &rig.grt,
            &c.spec,
            ProbePoint { node: c.proc, port },
            ProbeMode::Monitor,
        )
        .unwrap()
}

// ---- adapter mapping ---------------------------------------------------------

/// A wired monitor probe reports one deck whose reading carries the
/// opaque id triple, the RT-confirmed mode, declared availability,
/// bound generation, observed clock, latest status and cumulative
/// counters — all mapped into the observability vocabulary.
#[test]
fn decks_maps_installed_monitor_probe() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    rig.probe.wire_clock_mark().unwrap();

    // Before the first block: binding is commanded but not yet
    // confirmed — no status, no records, UNKNOWN clock.
    let decks = rig.probe.decks();
    assert_eq!(decks.len(), 1);
    let deck = &decks[0];
    assert!(deck.clock.is_unknown());
    assert_eq!(deck.probes.len(), 1);
    let r = &deck.probes[0];
    assert_eq!(r.probe, p.get());
    assert_eq!(r.node, c.proc.get());
    assert_eq!(r.port, c.proc_in.get());
    assert_eq!(r.recorded, 0);
    assert_eq!(r.last_status, None);

    block(&mut rig, &c);
    block(&mut rig, &c);

    let decks = rig.probe.decks();
    let deck = &decks[0];
    let mark = rig
        .probe
        .with_time_context(ruviform_audio::typed::probe::ProbeTimeContext::mark);
    assert_eq!(deck.clock.domain, mark.domain);
    assert_eq!(deck.clock.epoch, mark.epoch);
    assert!(!deck.clock.is_unknown());
    assert_eq!(deck.generation, 1);
    assert_eq!(deck.confidentiality, Confidentiality::Internal);

    let r = &deck.probes[0];
    assert_eq!(r.generation, 1);
    assert_eq!(r.mode, ruviform_observability::debug::ProbeMode::Monitor);
    assert_eq!(
        r.availability,
        ruviform_observability::debug::ProbeAvailability::Configured
    );
    assert_eq!(
        r.last_status,
        Some(ruviform_observability::debug::ProbeStatusKind::Installed)
    );
    // Two blocks → two metrics + the Installed status = 3 records.
    assert_eq!(r.recorded, 3);
    assert_eq!(r.dropped, 0);
    assert_eq!(r.lost_blocks, 0);
}

/// Undeclared-but-bound points keep their `Inferred` class — the
/// adapter must not flatten availability into a boolean.
#[test]
fn decks_preserves_inferred_availability() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    // `sink_in` is connected (a slot is bound) but carries no
    // `ProbePoint` declaration → `Inferred`, not `Configured`.
    let p = rig
        .probe
        .install(
            &rig.grt,
            &c.spec,
            ProbePoint {
                node: c.sink,
                port: c.sink_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    block(&mut rig, &c);
    let r = &rig.probe.decks()[0].probes[0];
    assert_eq!(r.probe, p.get());
    assert_eq!(r.node, c.sink.get());
    assert_eq!(r.port, c.sink_in.get());
    assert_eq!(
        r.availability,
        ruviform_observability::debug::ProbeAvailability::Inferred
    );
}

/// Off→Monitor and capture lifecycle statuses reconcile the
/// RT-confirmed mode and land in `last_status` with typed payloads.
#[test]
fn decks_tracks_mode_and_capture_statuses() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    block(&mut rig, &c);

    rig.probe.set_mode(p, ProbeMode::Off).unwrap();
    block(&mut rig, &c);
    let r = &rig.probe.decks()[0].probes[0];
    assert_eq!(
        r.mode,
        ruviform_observability::debug::ProbeMode::Off,
        "ModeChanged status must reconcile applied mode"
    );
    assert!(matches!(
        r.last_status,
        Some(ruviform_observability::debug::ProbeStatusKind::ModeChanged { .. })
    ));

    rig.probe.set_mode(p, ProbeMode::Monitor).unwrap();
    rig.probe
        .capture(p, CaptureConfig { max_frames: 96 })
        .unwrap();
    block(&mut rig, &c);
    let r = &rig.probe.decks()[0].probes[0];
    assert_eq!(r.mode, ruviform_observability::debug::ProbeMode::Capture);
    assert!(matches!(
        r.last_status,
        Some(ruviform_observability::debug::ProbeStatusKind::CaptureStarted { .. })
    ));

    // Budget (96 frames < 2×64-frame blocks) exhausts → stop status,
    // mode falls back to Monitor.
    block(&mut rig, &c);
    block(&mut rig, &c);
    let r = &rig.probe.decks()[0].probes[0];
    assert_eq!(r.mode, ruviform_observability::debug::ProbeMode::Monitor);
    assert!(matches!(
        r.last_status,
        Some(
            ruviform_observability::debug::ProbeStatusKind::CaptureStopped {
                reason: ruviform_observability::debug::ProbeStopReason::BudgetExhausted
            }
        )
    ));
}

/// `decks()` drains telemetry only — capture chunks stay queued for
/// `drain_events`, which delivers them without the inspection path
/// ever seeing PCM.
#[test]
fn decks_never_consumes_capture_chunks() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    rig.probe
        .capture(p, CaptureConfig { max_frames: 1_000 })
        .unwrap();
    block(&mut rig, &c);

    // Inspection drains telemetry but must leave chunks queued.
    let _ = rig.probe.decks();
    let mut captured = 0usize;
    let mut statuses = 0usize;
    let delivered = rig.probe.drain_events(|ev| match ev {
        ProbeEvent::Captured(_) => captured += 1,
        ProbeEvent::Status(_) => statuses += 1,
        _ => {}
    });
    assert!(captured >= 1, "capture chunk must survive decks()");
    assert!(delivered >= captured + statuses);
}

/// Republish + `on_publish` advances the bound generation; a probe
/// whose point stops resolving keeps its old generation (visible as
/// deck/reading skew) and reports the failure-class availability.
#[test]
fn rebind_updates_generation_and_preserves_failure_class() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    block(&mut rig, &c);
    let _ = rig.probe.decks();

    // Same spec republished as generation 2, then rebind — the
    // point resolves again and the reading follows the new
    // generation. The factory hands out one processor per compile.
    f.pending.insert(c.proc, gain2());
    republish(&mut rig, &c.spec, &mut f);
    let receipt = rig.probe.on_publish(&rig.grt, &c.spec).unwrap();
    assert_eq!(receipt.unbound, Vec::<ProbeId>::new());
    block(&mut rig, &c);
    let deck = &rig.probe.decks()[0];
    assert_eq!(deck.generation, 2);
    assert_eq!(deck.probes[0].generation, 2);
    assert!(deck.probes.iter().all(|r| r.generation == deck.generation));

    // A spec where `proc` is isolated keeps the same node/port IDs
    // (per-builder issuance) but leaves `proc_in` with no backing
    // slot: the point stops resolving, the binding drops at commit,
    // and the reading must carry the failure class plus the stale
    // generation — nothing is flattened to "fine".
    let c2 = probe_chain_inner(false);
    let mut f2 = MockFactory::default();
    f2.pending.insert(c2.proc, gain2());
    republish(&mut rig, &c2.spec, &mut f2);
    let receipt = rig.probe.on_publish(&rig.grt, &c2.spec).unwrap();
    assert!(receipt.unbound.contains(&p));
    // The published plan now belongs to `c2` — feed its source port.
    block(&mut rig, &c2);
    let deck = &rig.probe.decks()[0];
    let r = &deck.probes[0];
    assert_eq!(deck.generation, 3);
    assert_eq!(r.generation, 2, "unresolved probe keeps old binding gen");
    assert_ne!(r.generation, deck.generation);
    assert_eq!(
        r.availability,
        ruviform_observability::debug::ProbeAvailability::Unavailable,
        "slotless audio port must keep its failure class"
    );
    assert!(matches!(
        r.last_status,
        Some(
            ruviform_observability::debug::ProbeStatusKind::Unbound
                | ruviform_observability::debug::ProbeStatusKind::RebindDropped
        )
    ));
}

// ---- inspector / runner wiring ----------------------------------------------

fn inspect_probes(
    inspector: &mut DebugInspector,
) -> ruviform_observability::debug::ProbesInspection {
    let snap = inspector
        .inspect(
            &InspectionRequest {
                targets: vec![InspectionTarget::Probes],
                detail: InspectionDetail::Verbose,
                disclosure: Confidentiality::Internal,
                budget: ruviform_observability::debug::InspectionBudget::default(),
            },
            &InspectionInputs::default(),
            Instant::now(),
        )
        .unwrap();
    snap.sections
        .iter()
        .find_map(|s| match s {
            InspectionSection::Probes(p) => Some(p.clone()),
            _ => None,
        })
        .expect("Probes section")
}

/// `DebugInspector::add_probe_source` accepts the adapter and `inspect`
/// produces a populated Probes section — the host-facing wiring path.
#[test]
fn inspector_registration_produces_probes_section() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    rig.probe.wire_clock_mark().unwrap();
    block(&mut rig, &c);

    let mut inspector = DebugInspector::new(DebugConfig::default());
    inspector.add_probe_source(Box::new(rig.probe.clone()));
    // Control continues through the original handle while the
    // registered clone serves inspections.
    let probes = inspect_probes(&mut inspector);
    assert!(matches!(probes.status, SectionStatus::Ok));
    assert_eq!(probes.decks.len(), 1);
    let deck = &probes.decks[0];
    assert_eq!(deck.probes.len(), 1);
    let entry = &deck.probes[0];
    assert_eq!(entry.probe, p.get());
    assert_eq!(entry.node, c.proc.get());
    assert_eq!(entry.port, c.proc_in.get());
    let counters = entry.counters.expect("Verbose detail carries counters");
    assert!(counters.recorded >= 2);
    assert_eq!(counters.dropped, 0);
    assert_eq!(counters.lost_blocks, 0);
}

/// `ObservationRunner::debug_mut()` is the composed wiring path —
/// the same adapter registers into the runner-owned inspector.
#[test]
fn runner_debug_mut_registers_adapter() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = install_monitor(&rig, &c, c.proc_in);
    block(&mut rig, &c);

    let mut runner = ObservationRunner::new();
    runner
        .prepare(&ObservabilityConfig {
            collection: CollectorConfig {
                poll_interval: Duration::from_millis(10),
                ..CollectorConfig::default()
            },
            diagnostics: None,
            debug: Some(DebugSettings::new(DebugConfig::default())),
        })
        .unwrap();
    runner.activate().unwrap();
    runner
        .debug_mut()
        .unwrap()
        .add_probe_source(Box::new(rig.probe.clone()));

    let snap = runner
        .debug_mut()
        .unwrap()
        .inspect(
            &InspectionRequest {
                targets: vec![InspectionTarget::Probes],
                detail: InspectionDetail::Standard,
                disclosure: Confidentiality::Internal,
                budget: ruviform_observability::debug::InspectionBudget::default(),
            },
            &InspectionInputs::default(),
            Instant::now(),
        )
        .unwrap();
    let probes = snap
        .sections
        .iter()
        .find_map(|s| match s {
            InspectionSection::Probes(p) => Some(p),
            _ => None,
        })
        .expect("Probes section");
    assert_eq!(probes.decks.len(), 1);
    assert_eq!(probes.decks[0].probes[0].probe, p.get());
    // Standard detail: no counters — identity/classification only.
    assert!(probes.decks[0].probes[0].counters.is_none());
}

/// Point availability classification at install time is preserved
/// through the adapter — `Unavailable` points are rejected by
/// `install` itself, so this asserts the surviving classes map
/// 1:1 rather than collapsing.
#[test]
fn availability_classes_map_one_to_one() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);

    // proc_in/proc_out carry ProbePoint declarations → Configured.
    let p_in = install_monitor(&rig, &c, c.proc_in);
    let p_out = install_monitor(&rig, &c, c.proc_out);
    block(&mut rig, &c);
    let deck = &rig.probe.decks()[0];
    assert_eq!(deck.probes.len(), 2);
    for r in &deck.probes {
        assert_eq!(
            r.availability,
            ruviform_observability::debug::ProbeAvailability::Configured
        );
    }
    assert!(deck.probes.iter().any(|r| r.probe == p_in.get()));
    assert!(deck.probes.iter().any(|r| r.probe == p_out.get()));

    // A point on a port with no backing slot is rejected at install —
    // the adapter surfaces the typed error, never a silent binding.
    // (sink_in belongs to the sink node; its id is not in the
    // fixture, so exercise the error path via an unknown port on the
    // proc node is impossible — PortId space is issued. Instead
    // assert the documented contract: install errors are typed.)
    let err = rig.probe.install(
        &rig.grt,
        &c.spec,
        ProbePoint {
            node: c.proc,
            port: c.sink_in, // proc has no such port → Unknown-class point
        },
        ProbeMode::Monitor,
    );
    assert!(err.is_err(), "cross-node port must not install");
}
