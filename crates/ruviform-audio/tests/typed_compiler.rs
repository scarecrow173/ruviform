//! G2-01/G2-02 — typed compiler evidence: deterministic schedules
//! and bounded buffer mappings from valid multi-bus DAGs; rejection
//! of every unsupported/overflow/invalid condition *before*
//! publication (no partial plan escapes).

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use common::{
    MockDsp, MockFactory, alloc, chain, config, conn, merge, node, port, pref, stereo, validate,
};
use ruviform_audio::typed::{CompileError, Compiler, InstalledInfo};
use ruviform_graph::{ConnDef, Direction, EdgeDelay, GraphBuilder, NodeKind, PortRole};

fn compile_chain() -> ruviform_audio::typed::CompiledPlan {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, log)));
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .expect("chain compiles")
}

#[test]
fn g2_01_chain_compiles_to_deterministic_schedule_and_slots() {
    let plan = compile_chain().plan;
    // 2 audio output ports bound → 2 producer slots (src.out,
    // proc.out; input ports consume in place, no scratch needed).
    assert_eq!(plan.slots.len(), 2, "one slot per audio output port");
    assert_eq!(plan.schedule.len(), 3);
    // Schedule is the deterministic topo order src → proc → sink.
    let c = chain();
    assert_eq!(plan.schedule[0].node, c.src);
    assert_eq!(plan.schedule[1].node, c.proc);
    assert_eq!(plan.schedule[2].node, c.sink);
    // Boundary tables: source output is the graph input; sink input
    // is the graph output.
    assert_eq!(plan.inputs.len(), 1);
    assert_eq!(plan.outputs.len(), 1);
    assert_eq!(plan.total_latency, 0);
}

#[test]
fn g2_01_repeated_compile_is_byte_identical() {
    let a = format!("{:?}", compile_chain().plan);
    let b = format!("{:?}", compile_chain().plan);
    assert_eq!(a, b, "same input → byte-identical plan dump");
}

#[test]
fn g2_01_fan_in_gets_scratch_slot() {
    let m = merge();
    let g = validate(&m.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        m.a,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    factory.pending.insert(
        m.b,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let plan = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .expect("merge compiles")
        .plan;
    // 4 producer slots + 1 scratch for the two-edge merge input.
    assert_eq!(plan.slots.len(), 5);
    let sink = &plan.schedule[4];
    assert_eq!(sink.node, m.sink);
    assert_eq!(sink.inputs[0].edges.len(), 2);
    assert!(sink.inputs[0].scratch.is_some());
}

#[test]
fn g2_02_declared_edge_delay_rejected() {
    // Rebuild the chain with a Declared delay on the src→proc edge —
    // validator accepts it (non-cycle), the M2 compiler must not.
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (proc, pp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (sink, kp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    let mut d = conn(pref(src, sp[0]), pref(proc, pp[0]));
    d.delay = EdgeDelay::Declared;
    b.connect(&d);
    b.connect(&ConnDef {
        from: pref(proc, pp[1]),
        to: pref(sink, kp[0]),
        delay: EdgeDelay::Zero,
        channel_map: None,
    });
    let spec = b.build();
    let g = validate(&spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        proc,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let err = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::UnsupportedEdgeDelay { .. }));
}

#[test]
fn g2_02_processor_kind_without_instance_rejected() {
    let c = chain();
    let g = validate(&c.spec);
    // Force the Processor node to Boundary — compiler must reject it
    // as MissingProcessor before publication.
    let mut factory = MockFactory {
        boundary: [c.proc].into_iter().collect(),
        ..Default::default()
    };
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let err = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::MissingProcessor { node } if node == c.proc));
}

#[test]
fn g2_02_factory_failure_propagates_and_consumes_nothing() {
    let c = chain();
    let g = validate(&c.spec);
    let mut factory = MockFactory {
        fail: [c.proc].into_iter().collect(),
        ..Default::default()
    };
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let err = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::Factory { node, .. } if node == c.proc));
}

#[test]
fn g2_02_slot_and_processor_overflows_rejected() {
    let c = chain();
    let g = validate(&c.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        c.proc,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    // max_slots = 1 < 2 producer slots needed.
    let mut cfg = config(256);
    cfg.max_slots = 1;
    let err = Compiler::new(cfg)
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::SlotOverflow { .. }));
    // max_processors = 0.
    let mut cfg = config(256);
    cfg.max_processors = 0;
    let mut factory = MockFactory::default();
    factory.pending.insert(
        c.proc,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let allocator = alloc(0);
    let err = Compiler::new(cfg)
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::ProcessorOverflow { .. }));
}

#[test]
fn g2_02_delay_overflow_rejected() {
    let c = chain();
    let g = validate(&c.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        c.proc,
        Box::new(MockDsp::new(64, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let mut cfg = config(256);
    cfg.max_delay_frames = 32; // 64 > 32
    let err = Compiler::new(cfg)
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .err()
        .unwrap();
    assert!(matches!(err, CompileError::DelayOverflow { .. }));
}

#[test]
fn g2_02_channel_mapped_edge_binds_map() {
    let m = common::mapped();
    let g = validate(&m.spec);
    let mut factory = MockFactory::default();
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let plan = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .expect("mapped edge compiles")
        .plan;
    let sink = plan
        .schedule
        .iter()
        .find(|n| n.node == m.sink)
        .expect("sink scheduled");
    assert_eq!(sink.inputs[0].edges[0].map, Some(m.map.clone()));
    // A channel-mapped edge always materializes through a scratch slot.
    assert!(sink.inputs[0].scratch.is_some());
}

#[test]
fn g2_02_reuse_keeps_slot_and_skips_install() {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    // First compile installs.
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    let registry = BTreeMap::new();
    let allocator = alloc(16);
    let first = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &allocator, 1)
        .expect("first compile");
    assert_eq!(first.installs.len(), 1);
    let pslot = first.installs[0].pslot;

    // Second compile with the instance live + reuse policy → Reuse,
    // same pslot, no install.
    let mut registry = BTreeMap::new();
    registry.insert(c.proc, InstalledInfo { pslot, latency: 0 });
    let mut factory = MockFactory {
        reuse: true,
        ..Default::default()
    };
    let alloc2 = first.alloc_after.clone();
    let second = Compiler::new(config(256))
        .compile(&g, &mut factory, &registry, &alloc2, 2)
        .expect("second compile");
    assert_eq!(second.installs.len(), 0);
    assert_eq!(second.removes, []);
    assert_eq!(
        second.plan.schedule[1].pslot,
        Some(pslot),
        "reused instance keeps its stable slot"
    );
}

#[test]
fn g2_02_removed_node_is_scheduled_for_removal() {
    // Install two processors from the merge graph, commit the
    // registry, then compile the plain chain against it — both
    // merge-graph processors must be scheduled for removal and
    // their slots released.
    let m = merge();
    let g_merge = validate(&m.spec);
    let mut factory = MockFactory::default();
    for n in [m.a, m.b] {
        factory.pending.insert(
            n,
            Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
        );
    }
    let empty_alloc = alloc(16);
    let first = Compiler::new(config(256))
        .compile(&g_merge, &mut factory, &BTreeMap::new(), &empty_alloc, 1)
        .expect("first compile");
    let mut registry = BTreeMap::new();
    for inst in &first.installs {
        registry.insert(
            inst.node,
            InstalledInfo {
                pslot: inst.pslot,
                latency: inst.latency,
            },
        );
    }
    let c = chain();
    let g_chain = validate(&c.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        c.proc,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let second = Compiler::new(config(256))
        .compile(&g_chain, &mut factory, &registry, &first.alloc_after, 2)
        .expect("second compile");
    assert_eq!(second.removes.len(), 2);
    // Every removed slot was released back for reuse.
    for &(_, pslot) in &second.removes {
        assert!(second.alloc_after.free.contains(&pslot));
    }
}
