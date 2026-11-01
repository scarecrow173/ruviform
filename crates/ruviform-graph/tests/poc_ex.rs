//! `PoC` acceptance tests for Issue #184 Migration M1 — mirrors the
//! `ex_001…ex_006` scenarios in the approved design
//! (`docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`).

use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_graph::{
    Activation, Availability, ChannelMap, ConnDef, Direction, EdgeDelay, GraphBuilder,
    GraphProject, GraphSpec, Multiplicity, NodeDef, NodeId, NodeKind, PortCapability, PortDef,
    PortFormat, PortId, PortRef, PortRole, PortSpec, PrepareError, SignalDomain, ValidationError,
    ValidationErrors, Validator,
};

// ---------- helpers ----------

fn fmt(rate: u32, layout: ChannelLayout) -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(rate).unwrap(),
        layout,
        sample_format: SampleFormat::F32,
    }
}

fn stereo() -> PortFormat {
    PortFormat::Audio(fmt(48_000, ChannelLayout::Stereo))
}

fn port(direction: Direction, role: PortRole, format: PortFormat) -> PortDef {
    PortDef {
        direction,
        role,
        format,
        multiplicity: Multiplicity::unbounded(0),
        capabilities: Vec::new(),
        availability: Availability::Always,
        activation: Activation::Enabled,
    }
}

fn node(kind: NodeKind, ports: Vec<PortDef>) -> NodeDef {
    NodeDef { kind, ports }
}

fn conn(from: PortRef, to: PortRef) -> ConnDef {
    ConnDef {
        from,
        to,
        delay: EdgeDelay::Zero,
        channel_map: None,
    }
}

fn pref(node: NodeId, port: PortId) -> PortRef {
    PortRef { node, port }
}

fn has_error(errors: &ValidationErrors, pred: impl Fn(&ValidationError) -> bool) -> bool {
    errors.errors().iter().any(pred)
}

/// Issues IDs outside a spec's range via a scratch builder — simulates
/// stale/foreign ID references (a real misuse pattern the validator
/// must reject, not a fabrication path inside the spec).
fn foreign_ids(node_count: usize) -> (NodeId, PortId) {
    let mut b = GraphBuilder::new();
    let foreign_port = port(Direction::Output, PortRole::Main, stereo());
    let mut last = b.add_node(&node(NodeKind::Processor, vec![foreign_port.clone()]));
    for _ in 0..node_count {
        last = b.add_node(&node(NodeKind::Processor, vec![foreign_port.clone()]));
    }
    (last.0, last.1[0])
}

/// Builds the canonical Source → Processor → Sink valid graph used by
/// several examples.
fn build_chain() -> (
    GraphSpec,
    NodeId,
    NodeId,
    NodeId,
    Vec<PortId>,
    Vec<PortId>,
    Vec<PortId>,
) {
    let mut b = GraphBuilder::new();
    let (src, src_ports) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (proc, proc_ports) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (sink, sink_ports) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    b.connect(&conn(pref(src, src_ports[0]), pref(proc, proc_ports[0])));
    b.connect(&conn(pref(proc, proc_ports[1]), pref(sink, sink_ports[0])));
    (
        b.build(),
        src,
        proc,
        sink,
        src_ports,
        proc_ports,
        sink_ports,
    )
}

// ---------- ex_001: representation ----------

#[test]
fn poc_ex_001_port_attributes_are_orthogonal_and_preserved() {
    let mut b = GraphBuilder::new();
    let (_n, ids) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            PortDef {
                direction: Direction::Output,
                role: PortRole::Main,
                format: stereo(),
                multiplicity: Multiplicity::exactly(1),
                capabilities: vec![PortCapability::LatencyReporting, PortCapability::ProbePoint],
                availability: Availability::Always,
                activation: Activation::Enabled,
            },
            PortDef {
                direction: Direction::Input,
                role: PortRole::Aux,
                format: PortFormat::Audio(fmt(96_000, ChannelLayout::Mono)),
                multiplicity: Multiplicity::unbounded(0),
                capabilities: vec![PortCapability::DelayCompensation],
                availability: Availability::Optional,
                activation: Activation::Disabled,
            },
            PortDef {
                direction: Direction::Input,
                role: PortRole::Sidechain,
                format: PortFormat::TimedEvent,
                multiplicity: Multiplicity::new(0, 4).unwrap(),
                capabilities: Vec::new(),
                availability: Availability::Always,
                activation: Activation::Enabled,
            },
            PortDef {
                direction: Direction::Output,
                role: PortRole::Send,
                format: PortFormat::Control,
                multiplicity: Multiplicity::unbounded(0),
                capabilities: Vec::new(),
                availability: Availability::Always,
                activation: Activation::Enabled,
            },
            PortDef {
                direction: Direction::Input,
                role: PortRole::Return,
                format: PortFormat::Audio(fmt(48_000, ChannelLayout::FivePointOne)),
                multiplicity: Multiplicity::exactly(2),
                capabilities: Vec::new(),
                availability: Availability::Optional,
                activation: Activation::Enabled,
            },
        ],
    ));
    let spec = b.build();
    let ports = spec.nodes()[0].ports();
    assert_eq!(ports.len(), 5);
    assert_eq!(ports.iter().map(PortSpec::id).collect::<Vec<_>>(), ids);

    let main = &ports[0];
    assert_eq!(main.direction(), Direction::Output);
    assert_eq!(main.role(), PortRole::Main);
    assert_eq!(main.format().domain(), SignalDomain::Audio);
    assert_eq!(
        main.format(),
        PortFormat::Audio(fmt(48_000, ChannelLayout::Stereo))
    );
    assert_eq!(main.multiplicity(), Multiplicity::exactly(1));
    assert_eq!(
        main.capabilities(),
        &[PortCapability::LatencyReporting, PortCapability::ProbePoint]
    );
    assert_eq!(main.availability(), Availability::Always);
    assert_eq!(main.activation(), Activation::Enabled);
    assert_eq!(main.owner(), spec.nodes()[0].id());

    let aux = &ports[1];
    assert_eq!(aux.format().domain(), SignalDomain::Audio);
    assert_eq!(aux.availability(), Availability::Optional);
    assert_eq!(aux.activation(), Activation::Disabled);
    assert_eq!(aux.capabilities(), &[PortCapability::DelayCompensation]);

    let sc = &ports[2];
    assert_eq!(sc.format().domain(), SignalDomain::TimedEvent);
    assert_eq!(sc.role(), PortRole::Sidechain);
    assert_eq!(sc.multiplicity(), Multiplicity::new(0, 4).unwrap());

    let send = &ports[3];
    assert_eq!(send.format().domain(), SignalDomain::Control);
    assert_eq!(send.role(), PortRole::Send);
    assert_eq!(send.multiplicity().max, Multiplicity::UNBOUNDED);

    let ret = &ports[4];
    assert_eq!(ret.role(), PortRole::Return);
    assert_eq!(
        ret.format(),
        PortFormat::Audio(fmt(48_000, ChannelLayout::FivePointOne))
    );
    assert_eq!(ret.multiplicity(), Multiplicity::exactly(2));
    assert_eq!(ret.multiplicity().min, 2);
}

// ---------- ex_002: multi-bus topology ----------

#[test]
fn poc_ex_002_multi_bus_topology_validates() {
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (proc, pp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Input, PortRole::Sidechain, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Send, stereo()),
        ],
    ));
    let (side, kp) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Sidechain, stereo())],
    ));
    // Sink Main input takes fan-in 2 (merge); Aux input takes Send feed.
    let mut sink_in = port(Direction::Input, PortRole::Main, stereo());
    sink_in.multiplicity = Multiplicity::new(0, 2).unwrap();
    let (sink, sk) = b.add_node(&node(
        NodeKind::Sink,
        vec![sink_in, port(Direction::Input, PortRole::Aux, stereo())],
    ));

    b.connect(&conn(pref(src, sp[0]), pref(proc, pp[0]))); // Main
    b.connect(&conn(pref(side, kp[0]), pref(proc, pp[1]))); // independent Sidechain
    b.connect(&conn(pref(proc, pp[2]), pref(sink, sk[0]))); // Main out -> merge
    b.connect(&conn(pref(proc, pp[3]), pref(sink, sk[1]))); // Send branch
    let (extra, ep) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    b.connect(&conn(pref(extra, ep[0]), pref(sink, sk[0]))); // fan-in merge leg 2

    let spec = b.build();
    let validated = Validator::validate(&spec).expect("multi-bus topology must validate");
    assert_eq!(validated.spec().nodes().len(), 5);
    assert_eq!(validated.spec().connections().len(), 5);
    assert_eq!(validated.mapping().node_order().len(), 5);
    // Sidechain source feeds processor before it in topo order.
    let m = validated.mapping();
    assert!(m.node_index(side).unwrap().get() < m.node_index(proc).unwrap().get());
    assert!(m.node_index(proc).unwrap().get() < m.node_index(sink).unwrap().get());
}

// ---------- ex_003: per-connection rejections ----------

#[test]
#[allow(clippy::too_many_lines)]
fn poc_ex_003_connection_level_rejections() {
    // InvalidDirection: Input -> Input.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    b.connect(&conn(pref(a, ap[0]), pref(c, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::InvalidDirection { .. }
    )));

    // DomainMismatch: Audio output -> TimedEvent input.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::TimedEvent,
        )],
    ));
    b.connect(&conn(pref(a, ap[0]), pref(c, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::DomainMismatch { .. }
    )));

    // FormatMismatch: 48k -> 96k same layout.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(96_000, ChannelLayout::Stereo)),
        )],
    ));
    b.connect(&conn(pref(a, ap[0]), pref(c, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::FormatMismatch { .. }
    )));
    assert!(!has_error(&errors, |e| matches!(
        e,
        ValidationError::ImplicitConversionRequired { .. }
    )));

    // ImplicitConversionRequired: Stereo -> Mono, no map.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(48_000, ChannelLayout::Mono)),
        )],
    ));
    b.connect(&conn(pref(a, ap[0]), pref(c, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::ImplicitConversionRequired { .. }
    )));

    // InvalidChannelMap: source index out of range.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(48_000, ChannelLayout::Mono)),
        )],
    ));
    let mut d = conn(pref(a, ap[0]), pref(c, cp[0]));
    d.channel_map = Some(ChannelMap {
        entries: vec![(5, 0)], // src 5 >= 2 channels
    });
    b.connect(&d);
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::InvalidChannelMap { .. }
    )));

    // InvalidChannelMap: map on a non-audio (Control) connection.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, PortFormat::Control)],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, PortFormat::Control)],
    ));
    let mut d = conn(pref(a, ap[0]), pref(c, cp[0]));
    d.channel_map = Some(ChannelMap {
        entries: vec![(0, 0)],
    });
    b.connect(&d);
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::InvalidChannelMap { .. }
    )));

    // Positive control: explicit downmix map Stereo -> Mono validates.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(48_000, ChannelLayout::Mono)),
        )],
    ));
    let mut d = conn(pref(a, ap[0]), pref(c, cp[0]));
    d.channel_map = Some(ChannelMap {
        entries: vec![(0, 0), (1, 0)], // both sources may feed ch 0
    });
    b.connect(&d);
    let spec = b.build();
    // dst ch 0 covered once per entry — duplicate dst coverage is
    // rejected, so this must fail as InvalidChannelMap.
    assert!(Validator::validate(&spec).is_err());

    // A legal map covers each destination exactly once.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(48_000, ChannelLayout::Stereo)),
        )],
    ));
    let mut d = conn(pref(a, ap[0]), pref(c, cp[0]));
    d.channel_map = Some(ChannelMap {
        entries: vec![(1, 0), (0, 1)], // explicit channel swap
    });
    b.connect(&d);
    let spec = b.build();
    assert!(Validator::validate(&spec).is_ok());
}

// ---------- ex_004: referential / cycle / capacity / required ----------

#[test]
#[allow(clippy::too_many_lines)]
fn poc_ex_004_graph_level_rejections() {
    // MissingNode: endpoint node not in spec.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (_c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    let (ghost_node, _) = foreign_ids(2);
    b.connect(&conn(pref(a, ap[0]), pref(ghost_node, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::MissingNode { node, .. } if *node == ghost_node
    )));

    // MissingPort: endpoint port not in spec.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, _cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    let (_, ghost_port) = foreign_ids(2);
    b.connect(&conn(pref(a, ap[0]), pref(c, ghost_port)));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::MissingPort { port, .. } if *port == ghost_port
    )));

    // PortNotOnNode: real port claimed on the wrong node.
    let mut b = GraphBuilder::new();
    let (a, _ap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    // cp[0] is owned by node c but the endpoint claims node a.
    b.connect(&conn(pref(a, cp[0]), pref(c, cp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::PortNotOnNode { node, port, .. } if *node == a && *port == cp[0]
    )));

    // ZeroDelayCycle: A -> B -> A all zero.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    b.connect(&conn(pref(a, ap[1]), pref(c, cp[0])));
    b.connect(&conn(pref(c, cp[1]), pref(a, ap[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::ZeroDelayCycle { nodes } if nodes.contains(&a) && nodes.contains(&c)
    )));
    assert!(!has_error(&errors, |e| matches!(
        e,
        ValidationError::DelayedCycleUnsupported { .. }
    )));

    // DelayedCycleUnsupported: one declared-delay edge in the cycle.
    let mut b = GraphBuilder::new();
    let (a, ap) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (c, cp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    b.connect(&conn(pref(a, ap[1]), pref(c, cp[0])));
    let mut back = conn(pref(c, cp[1]), pref(a, ap[0]));
    back.delay = EdgeDelay::Declared;
    b.connect(&back);
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::DelayedCycleUnsupported { .. }
    )));
    assert!(!has_error(&errors, |e| matches!(
        e,
        ValidationError::ZeroDelayCycle { .. }
    )));

    // CapacityExceeded: output port max=1 with two outgoing conns.
    let mut b = GraphBuilder::new();
    let mut out = port(Direction::Output, PortRole::Main, stereo());
    out.multiplicity = Multiplicity::new(0, 1).unwrap();
    let (a, ap) = b.add_node(&node(NodeKind::Source, vec![out]));
    let (c, cp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    let (d, dp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    b.connect(&conn(pref(a, ap[0]), pref(c, cp[0])));
    b.connect(&conn(pref(a, ap[0]), pref(d, dp[0])));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::CapacityExceeded { port, have: 2, max: 1, .. } if *port == ap[0]
    )));

    // RequiredConnections: Always + Enabled input with min=1 unconnected.
    let mut b = GraphBuilder::new();
    let mut required_in = port(Direction::Input, PortRole::Main, stereo());
    required_in.multiplicity = Multiplicity::exactly(1);
    let (a, ap) = b.add_node(&node(NodeKind::Sink, vec![required_in]));
    let spec = b.build();
    let errors = Validator::validate(&spec).unwrap_err();
    assert!(has_error(&errors, |e| matches!(
        e,
        ValidationError::RequiredConnections { port, have: 0, min: 1 } if *port == ap[0]
    )));
    let _ = a;
}

// ---------- ex_005: revisions + stable↔runtime mapping ----------

#[test]
fn poc_ex_005_revisions_and_runtime_mapping() {
    let (spec, src, proc, sink, sp, _pp, _sk) = build_chain();
    let mut project = GraphProject::new();
    assert_eq!(project.requested_revision(), None);
    assert_eq!(project.published_revision(), None);

    let rev = project.prepare(spec).expect("valid spec prepares");
    assert_eq!(project.requested_revision(), Some(rev));
    assert_eq!(project.published_revision(), Some(rev));
    assert_eq!(rev.get(), 1);

    let published = project.published().unwrap();
    let mapping = published.graph().mapping();
    // Topo order: source < processor < sink at runtime indexes 0,1,2.
    assert_eq!(mapping.node_index(src).unwrap().get(), 0);
    assert_eq!(mapping.node_index(proc).unwrap().get(), 1);
    assert_eq!(mapping.node_index(sink).unwrap().get(), 2);
    // Reverse lookups.
    assert_eq!(mapping.node_id(mapping.node_index(src).unwrap()), Some(src));
    // Ports are insertion-ordered: source port is runtime port 0.
    assert_eq!(mapping.port_index(sp[0]).unwrap().get(), 0);
    assert_eq!(
        mapping.port_id(mapping.port_index(sp[0]).unwrap()),
        Some(sp[0])
    );
    let c1 = published.graph().spec().connections()[0].id();
    assert_eq!(mapping.connection_index(c1).unwrap().get(), 0);
    assert_eq!(
        mapping.connection_id(mapping.connection_index(c1).unwrap()),
        Some(c1)
    );

    // A failed prepare bumps requested but not published.
    let mut bad = GraphBuilder::new();
    let (a, ap) = bad.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    bad.connect(&conn(pref(a, ap[1]), pref(a, ap[0]))); // self-loop
    let err = project.prepare(bad.build()).unwrap_err();
    match err {
        PrepareError::Validation { revision, errors } => {
            assert_eq!(revision.get(), 2);
            assert!(has_error(&errors, |e| matches!(
                e,
                ValidationError::ZeroDelayCycle { .. }
            )));
        }
        other => panic!("unexpected prepare error: {other}"),
    }
    assert_eq!(project.requested_revision().unwrap().get(), 2);
    assert_eq!(project.published_revision().unwrap().get(), 1);
}

// ---------- ex_006: failed prep preserves + single-bus golden ----------

#[test]
fn poc_ex_006_failure_preserves_published_and_main_only_golden() {
    // Part 1: failed validation preserves the previously published graph.
    let (good, _, _, _, _, _, _) = build_chain();
    let mut project = GraphProject::new();
    project.prepare(good).unwrap();
    let old_nodes = project.published().unwrap().graph().spec().nodes().len();
    let old_conns = project
        .published()
        .unwrap()
        .graph()
        .spec()
        .connections()
        .len();

    let mut bad = GraphBuilder::new();
    let (a, ap) = bad.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (_c, cp) = bad.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    let (ghost, _) = foreign_ids(2);
    bad.connect(&conn(pref(a, ap[0]), pref(ghost, cp[0])));
    assert!(project.prepare(bad.build()).is_err());

    let published = project.published().unwrap();
    assert_eq!(published.revision().get(), 1);
    assert_eq!(published.graph().spec().nodes().len(), old_nodes);
    assert_eq!(published.graph().spec().connections().len(), old_conns);

    // Part 2: single-bus Main-only golden — the legacy
    // Source -> Processor -> Sink topology must map onto the IR
    // with fixed expected structure, endpoint IDs, and runtime order.
    let (spec, src, proc, sink, sp, pp, sk) = build_chain();
    let validated = Validator::validate(&spec).unwrap();

    assert_eq!(validated.spec().nodes().len(), 3);
    assert_eq!(validated.spec().connections().len(), 2);

    // Stable IDs issue from 1 in declaration order.
    assert_eq!(src.get(), 1);
    assert_eq!(proc.get(), 2);
    assert_eq!(sink.get(), 3);
    assert_eq!(sp[0].get(), 1);
    assert_eq!(pp[0].get(), 2);
    assert_eq!(pp[1].get(), 3);
    assert_eq!(sk[0].get(), 4);

    let conns = validated.spec().connections();
    assert_eq!(conns[0].id().get(), 1);
    assert_eq!(conns[0].from(), pref(src, sp[0]));
    assert_eq!(conns[0].to(), pref(proc, pp[0]));
    assert_eq!(conns[1].id().get(), 2);
    assert_eq!(conns[1].from(), pref(proc, pp[1]));
    assert_eq!(conns[1].to(), pref(sink, sk[0]));

    // Runtime node order equals the linear chain.
    let order = validated.mapping().node_order();
    assert_eq!(order, &[src, proc, sink]);
}
