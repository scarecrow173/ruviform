//! Shared fixtures for the Issue #195 typed-runtime integration
//! tests: graph builders, a stateful mock DSP, and a configurable
//! `NodeFactory`.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use ruviform_audio::typed::{
    CompilerConfig, DEFAULT_ALIGNMENT, FactoryError, GraphProcessor, InstalledInfo, NodeEvent,
    NodeFactory, NodeIo, PrepareOutcome,
};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_graph::{
    Activation, Availability, ChannelMap, ConnDef, Direction, EdgeDelay, GraphBuilder, GraphSpec,
    Multiplicity, NodeDef, NodeId, NodeKind, NodeSpec, PortDef, PortFormat, PortId, PortRef,
    PortRole, ValidatedGraph, Validator,
};
use ruviform_rt::RtContext;

pub fn fmt(rate: u32, layout: ChannelLayout) -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(rate).unwrap(),
        layout,
        sample_format: SampleFormat::F32,
    }
}

pub fn stereo() -> PortFormat {
    PortFormat::Audio(fmt(48_000, ChannelLayout::Stereo))
}

pub fn port(direction: Direction, role: PortRole, format: PortFormat) -> PortDef {
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

pub fn node(kind: NodeKind, ports: Vec<PortDef>) -> NodeDef {
    NodeDef { kind, ports }
}

pub fn conn(from: PortRef, to: PortRef) -> ConnDef {
    ConnDef {
        from,
        to,
        delay: EdgeDelay::Zero,
        channel_map: None,
    }
}

pub fn pref(node: NodeId, port: PortId) -> PortRef {
    PortRef { node, port }
}

pub fn validate(spec: &GraphSpec) -> ValidatedGraph {
    Validator::validate(spec).expect("fixture graph must validate")
}

/// Source(out stereo) → Processor(in/out stereo) → Sink(in stereo).
pub struct Chain {
    pub spec: GraphSpec,
    pub src: NodeId,
    pub proc: NodeId,
    pub sink: NodeId,
    pub src_out: PortId,
    pub proc_in: PortId,
    pub proc_out: PortId,
    pub sink_in: PortId,
}

pub fn chain() -> Chain {
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
    b.connect(&conn(pref(src, sp[0]), pref(proc, pp[0])));
    b.connect(&conn(pref(proc, pp[1]), pref(sink, kp[0])));
    Chain {
        spec: b.build(),
        src,
        proc,
        sink,
        src_out: sp[0],
        proc_in: pp[0],
        proc_out: pp[1],
        sink_in: kp[0],
    }
}

/// Two parallel paths with different own-latencies merging into a
/// fan-in sink: `src_a → a(L_a) ⇢` and `src_b → b(L_b) ⇢` merge.
pub struct Merge {
    pub spec: GraphSpec,
    pub src_a: NodeId,
    pub src_b: NodeId,
    pub a: NodeId,
    pub b: NodeId,
    pub sink: NodeId,
    pub a_out: PortId,
    pub b_out: PortId,
    pub sink_in: PortId,
    /// Boundary input ports (the processor-less Sources' outputs).
    pub src_a_out: PortId,
    pub src_b_out: PortId,
}

pub fn merge() -> Merge {
    let mut b = GraphBuilder::new();
    let (src_a, sap) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (src_b, sbp) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (a, ap) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (bb, bp) = b.add_node(&node(
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
    b.connect(&conn(pref(src_a, sap[0]), pref(a, ap[0])));
    b.connect(&conn(pref(src_b, sbp[0]), pref(bb, bp[0])));
    b.connect(&conn(pref(a, ap[1]), pref(sink, kp[0])));
    b.connect(&conn(pref(bb, bp[1]), pref(sink, kp[0])));
    Merge {
        spec: b.build(),
        src_a,
        src_b,
        a,
        b: bb,
        sink,
        a_out: ap[1],
        b_out: bp[1],
        sink_in: kp[0],
        src_a_out: sap[0],
        src_b_out: sbp[0],
    }
}

/// Stereo→mono channel-mapped edge into a sink.
pub struct Mapped {
    pub spec: GraphSpec,
    pub src: NodeId,
    pub sink: NodeId,
    pub src_out: PortId,
    pub sink_in: PortId,
    pub map: Vec<(u16, u16)>,
}

pub fn mapped() -> Mapped {
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (sink, kp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(
            Direction::Input,
            PortRole::Main,
            PortFormat::Audio(fmt(48_000, ChannelLayout::Mono)),
        )],
    ));
    let mut d = conn(pref(src, sp[0]), pref(sink, kp[0]));
    let map = vec![(0u16, 0u16)]; // L → mono
    d.channel_map = Some(ChannelMap {
        entries: map.clone(),
    });
    b.connect(&d);
    Mapped {
        spec: b.build(),
        src,
        sink,
        src_out: sp[0],
        sink_in: kp[0],
        map,
    }
}

/// Stateful mock DSP: `state` accumulates per process call, making
/// instance identity observable through output and counters;
/// `log` records lifecycle evidence (`activate`/`deactivate`/`event`
/// are only ever called on the RT side — reads happen non-RT).
pub struct MockDsp {
    latency: u32,
    gain: f32,
    /// +1 per `process` call — the state-continuity witness.
    pub state: f32,
    /// `process` call count.
    pub calls: u64,
    /// Lifecycle log — non-RT readers only.
    pub log: Arc<Mutex<Vec<String>>>,
}

impl MockDsp {
    pub fn new(latency: u32, gain: f32, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            latency,
            gain,
            state: 0.0,
            calls: 0,
            log,
        }
    }
}

impl GraphProcessor for MockDsp {
    fn latency(&self) -> u32 {
        self.latency
    }

    fn activate(&mut self, _ctx: &RtContext) {
        self.log
            .lock()
            .unwrap()
            .push(format!("activate:state={}", self.state));
    }

    fn event(&mut self, ev: &NodeEvent, _ctx: &RtContext) {
        self.gain = ev.value;
        self.log
            .lock()
            .unwrap()
            .push(format!("event:gen={}:v={}", ev.generation, ev.value));
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        self.calls += 1;
        self.state += 1.0;
        let input = io.input(0);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for ch in 0..out.channels() {
                let Some(plane) = out.channel_mut(ch) else {
                    continue;
                };
                match input.as_ref().and_then(|v| v.channel(ch)) {
                    Some(src) => {
                        for (d, s) in plane.iter_mut().zip(src.iter()) {
                            // state offset makes instance identity
                            // visible in the produced samples.
                            *d = s * self.gain + self.state;
                        }
                    }
                    None => plane.fill(self.state),
                }
            }
        }
    }

    fn deactivate(&mut self, _ctx: &RtContext) {
        self.log
            .lock()
            .unwrap()
            .push(format!("deactivate:state={}", self.state));
    }
}

/// Configurable factory: `Processor` nodes draw from `pending`;
/// `boundary`/`fail`/`reuse` inject the named behaviors. Non-
/// `Processor` nodes (Source/Sink) default to `Boundary`.
#[derive(Default)]
pub struct MockFactory {
    /// Instances prepared per node (drained on `New`).
    pub pending: BTreeMap<NodeId, Box<dyn GraphProcessor>>,
    /// Nodes forced to `PrepareOutcome::Boundary`.
    pub boundary: BTreeSet<NodeId>,
    /// Nodes forced to a `FactoryError`.
    pub fail: BTreeSet<NodeId>,
    /// Reuse the installed instance when one exists.
    pub reuse: bool,
    /// `prepare` call log — the install/reuse evidence.
    pub calls: Arc<Mutex<Vec<NodeId>>>,
}

impl NodeFactory for MockFactory {
    fn prepare(
        &mut self,
        node: &NodeSpec,
        existing: Option<&InstalledInfo>,
    ) -> Result<PrepareOutcome, FactoryError> {
        self.calls.lock().unwrap().push(node.id());
        if self.fail.contains(&node.id()) {
            return Err(FactoryError("injected factory failure".into()));
        }
        if node.kind() != NodeKind::Processor || self.boundary.contains(&node.id()) {
            return Ok(PrepareOutcome::Boundary);
        }
        if self.reuse && existing.is_some() {
            return Ok(PrepareOutcome::Reuse);
        }
        self.pending
            .remove(&node.id())
            .map(PrepareOutcome::New)
            .ok_or_else(|| FactoryError(format!("no pending processor for {:?}", node.id())))
    }
}

/// Empty processor-slot allocator for standalone compiles.
pub fn alloc(max: usize) -> ruviform_audio::typed::SlotAllocator {
    ruviform_audio::typed::SlotAllocator {
        free: BTreeSet::new(),
        next: 0,
        max,
    }
}

pub fn config(max_frames: usize) -> CompilerConfig {
    CompilerConfig {
        max_frames,
        alignment: DEFAULT_ALIGNMENT,
        max_slots: 64,
        max_channels: 8,
        max_delay_lines: 16,
        max_delay_frames: 4096,
        max_processors: 16,
    }
}

/// Graph output drain: reads the mono/stereo boundary slot after a
/// `process` call.
pub fn drain(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId) -> Vec<Vec<f32>> {
    let view = rt.output_reader(port).expect("output port bound");
    let mut out = Vec::with_capacity(usize::from(view.channels));
    for ch in 0..usize::from(view.channels) {
        out.push(view.data.channel(ch).expect("channel within spec").to_vec());
    }
    out
}

/// Graph input feed: writes `data[c][..frames]` per channel of the
/// boundary input slot.
pub fn feed(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId, data: &[&[f32]]) {
    let frames = data.first().map_or(0, |p| p.len());
    let mut slot = rt.input_writer(port, frames).expect("input port bound");
    for (ch, src) in data.iter().enumerate() {
        let plane = slot.data.channel_mut(ch).expect("channel within spec");
        plane[..src.len()].copy_from_slice(src);
    }
}
