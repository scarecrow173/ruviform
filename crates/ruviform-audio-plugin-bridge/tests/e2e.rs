//! M4 evidence — the bridge driven through the real typed runtime:
//! multi-bus audio, legal physical-call partitioning at a loop
//! boundary, process-start-relative event offsets, and the
//! L0/L1-style capability declarations.

use std::sync::{Arc, Mutex};

use ruviform_audio::typed::{Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_plugin_bridge::{
    AttachConfig, Backend, CapabilityLevel, PluginNodeFactory, attach, capability_for,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity, NodeDef,
    NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
};
use ruviform_media_core::{
    DeliveryPolicy, MediaDuration, MediaRange, MediaTime, PlanGeneration, ScheduleBudget, TimeBase,
    TransportCommand, TransportPlanBuilder,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, ErasedPlugin, Event,
    EventKind, EventList, ParamEvent, ParamId, ParamInfo, ParamValue, Plugin, PluginDescriptor,
    PluginFactory, PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult,
    TransportInfo, erase_sendable,
};
use ruviform_plugin_host::{EndpointBounds, HostCommand, HostReply, PluginHost};
use ruviform_rt::RtContext;

const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.m4-rec",
    name: "M4 Recorder",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// Stereo main in + mono sidechain in / stereo main out + stereo aux
/// out — the Main+Aux/Sidechain・Mono/Stereo fixture.
const BUSES: [BusDescriptor; 4] = [
    BusDescriptor {
        direction: BusDirection::Input,
        layout: ChannelLayout::Stereo,
        name: "Main In",
        role: BusRole::Main,
        enabled: true,
    },
    BusDescriptor {
        direction: BusDirection::Input,
        layout: ChannelLayout::Mono,
        name: "Sidechain",
        role: BusRole::Sidechain,
        enabled: true,
    },
    BusDescriptor {
        direction: BusDirection::Output,
        layout: ChannelLayout::Stereo,
        name: "Main Out",
        role: BusRole::Main,
        enabled: true,
    },
    BusDescriptor {
        direction: BusDirection::Output,
        layout: ChannelLayout::Stereo,
        name: "Aux Out",
        role: BusRole::Aux,
        enabled: true,
    },
];

/// One physical `process` call as the plugin saw it.
#[derive(Debug)]
struct CallRec {
    frames: usize,
    offsets: Vec<u32>,
}

struct RecPlugin {
    calls: Arc<Mutex<Vec<CallRec>>>,
}

impl PluginProcessor for RecPlugin {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        self.calls.lock().unwrap().push(CallRec {
            frames: outputs.first().map_or(0, BufferView::frames),
            offsets: events.iter().map(Event::frame_offset).collect(),
        });
        for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
            for ch in 0..output.channels() {
                if let (Some(src), Some(dst)) = (
                    input.channel(ch.min(input.channels() - 1)),
                    output.channel_mut(ch),
                ) {
                    for (s, d) in src.iter().zip(dst.iter_mut()) {
                        *d = *s * 2.0;
                    }
                }
            }
        }
        ProcessResult::CONTINUE
    }
}

impl Plugin for RecPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }
    fn params(&self) -> &[ParamInfo] {
        &[]
    }
    fn buses(&self) -> &[BusDescriptor] {
        &BUSES
    }
}

struct RecFactory {
    calls: Arc<Mutex<Vec<CallRec>>>,
}

impl PluginFactory for RecFactory {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }
    fn create(&self) -> Box<dyn ErasedPlugin> {
        ruviform_plugin_core::erase(RecPlugin {
            calls: Arc::clone(&self.calls),
        })
    }
    fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
        Some(erase_sendable(RecPlugin {
            calls: Arc::clone(&self.calls),
        }))
    }
}

struct Rig {
    _host: PluginHost,
    _runtime: GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    control: ruviform_audio_plugin_bridge::PluginNodeControl,
    calls: Arc<Mutex<Vec<CallRec>>>,
    src_outs: [PortId; 2],
    sink_ins: [PortId; 2],
}

fn fmt() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
}

fn port(direction: Direction, role: ruviform_graph::PortRole, format: PortFormat) -> PortDef {
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

/// Drives a M4 fixture to `Processing`, attaches it into a
/// `Source(2 outs) → plugin → Sink(2 ins)` graph and returns the rig.
#[expect(clippy::too_many_lines)]
fn build_rig(min_frames: u32, max_frames: u32) -> Rig {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut host = PluginHost::new();
    host.register_factory(Box::new(RecFactory {
        calls: Arc::clone(&calls),
    }));
    let HostReply::Instantiated { handle } = host
        .execute(HostCommand::Instantiate {
            plugin_id: DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    else {
        panic!("Instantiate")
    };
    host.execute(HostCommand::Initialize { handle }).unwrap();
    let process_config =
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), min_frames, max_frames).unwrap();
    host.execute(HostCommand::Activate {
        handle,
        config: process_config,
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    let attached = attach(
        &mut host,
        handle,
        &AttachConfig {
            buses: BUSES.to_vec(),
            port_format: fmt(),
            bounds: EndpointBounds {
                max_frames,
                channels: 2,
            },
            process_config,
            frame_base: TimeBase::sample_rate(48_000).unwrap(),
            policy: DeliveryPolicy::Exact,
            budget: ScheduleBudget::default(),
        },
    )
    .expect("attach");

    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&NodeDef {
        kind: NodeKind::Source,
        ports: vec![
            port(
                Direction::Output,
                ruviform_graph::PortRole::Main,
                PortFormat::Audio(fmt()),
            ),
            port(
                Direction::Output,
                ruviform_graph::PortRole::Sidechain,
                PortFormat::Audio(AudioFormat {
                    layout: ChannelLayout::Mono,
                    ..fmt()
                }),
            ),
        ],
    });
    let (proc, pp) = b.add_node(&NodeDef {
        kind: NodeKind::Processor,
        ports: attached.ports.clone(),
    });
    let (sink, kp) = b.add_node(&NodeDef {
        kind: NodeKind::Sink,
        ports: vec![
            port(
                Direction::Input,
                ruviform_graph::PortRole::Main,
                PortFormat::Audio(fmt()),
            ),
            port(
                Direction::Input,
                ruviform_graph::PortRole::Aux,
                PortFormat::Audio(fmt()),
            ),
        ],
    });
    let conn = |from, to| ConnDef {
        from,
        to,
        delay: EdgeDelay::Zero,
        channel_map: None,
    };
    let pref = |node, port| PortRef { node, port };
    b.connect(&conn(pref(src, sp[0]), pref(proc, pp[0])));
    b.connect(&conn(pref(src, sp[1]), pref(proc, pp[1])));
    b.connect(&conn(pref(proc, pp[2]), pref(sink, kp[0])));
    b.connect(&conn(pref(proc, pp[3]), pref(sink, kp[1])));
    let g = Validator::validate(&b.build()).expect("graph validates");

    let config = CompilerConfig {
        max_frames: 512,
        alignment: DEFAULT_ALIGNMENT,
        max_slots: 64,
        max_channels: 8,
        max_delay_lines: 16,
        max_delay_frames: 4096,
        max_processors: 16,
    };
    let mut factory = PluginNodeFactory::new();
    factory.insert(proc, attached.node);
    let (mut runtime, rt) = GraphRuntime::new(&config, 8, 8);
    let compiled = Compiler::new(config)
        .compile(
            &g,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            runtime.generation() + 1,
        )
        .expect("compiles");
    runtime.publish(compiled).expect("publish");
    Rig {
        _host: host,
        _runtime: runtime,
        rt,
        control: attached.control,
        calls,
        src_outs: [sp[0], sp[1]],
        sink_ins: [kp[0], kp[1]],
    }
}

fn feed(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId, data: &[&[f32]]) {
    let frames = data.first().map_or(0, |p| p.len());
    let mut slot = rt.input_writer(port, frames).expect("input bound");
    for (ch, src) in data.iter().enumerate() {
        let plane = slot.data.channel_mut(ch).expect("channel");
        plane[..src.len()].copy_from_slice(src);
    }
}

fn drain(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId) -> Vec<Vec<f32>> {
    let view = rt.output_reader(port).expect("output bound");
    (0..usize::from(view.channels))
        .map(|ch| view.data.channel(ch).expect("channel").to_vec())
        .collect()
}

/// M4-02 — Main+Sidechain inputs・Main+Aux outputs route intact
/// through the typed graph into the hosted plugin.
#[test]
#[expect(clippy::cast_precision_loss)]
fn multi_bus_audio_routes_and_processes() {
    let mut rig = build_rig(1, 512);
    let ctx = RtContext::mint();
    let main_in: Vec<f32> = (0..256).map(|f| f as f32).collect();
    let sc_in = vec![0.25f32; 256];
    feed(
        &mut rig.rt,
        rig.src_outs[0],
        &[&main_in.clone(), &main_in.clone()],
    );
    feed(&mut rig.rt, rig.src_outs[1], &[&sc_in]);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "one physical call for 256<=512");
    assert_eq!(calls[0].frames, 256);
    drop(calls);

    let main = drain(&mut rig.rt, rig.sink_ins[0]);
    for plane in &main {
        for (f, &v) in plane.iter().enumerate().take(256) {
            assert!((v - f as f32 * 2.0).abs() < f32::EPSILON, "main gain 2x");
        }
    }
    let aux = drain(&mut rig.rt, rig.sink_ins[1]);
    for plane in &aux {
        assert!(
            plane[..256].iter().all(|&v| (v - 0.5).abs() < f32::EPSILON),
            "aux echoes sidechain 0.25*2"
        );
    }
}

/// M4-03 — 256-frame callback + `Loop=100` + `min=128`: every loop
/// lap is ≤100 frames, so a conforming implementation makes **zero**
/// physical calls for them (a 100-frame process call is the failing
/// implementation) and reports each undeliverable span. The assertion
/// is deterministic: no call outside `[min,max]` may ever occur.
#[test]
fn loop_100_min_128_zero_violating_calls() {
    let mut rig = build_rig(128, 256);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    rig.control
        .request(
            TransportCommand::SetLoop(MediaRange::new(
                MediaTime::new(0, base),
                MediaDuration::new(100, base),
            )),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert!(
        calls.iter().all(|c| (128..=256).contains(&c.frames)),
        "min/max-violating process call detected: {calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c.frames == 100),
        "a 100-frame process call is the disqualifying implementation"
    );
    let reports = rig.control.drain_reports();
    assert!(
        reports.iter().any(|r| matches!(
            r,
            ruviform_audio_plugin_bridge::NodeReport::UndeliverableSpan { .. }
        )),
        "undeliverable loop laps must be audited, not silent: {reports:?}"
    );
}

/// M4-03 companion — a *partially* legal callback: `Loop=200` splits
/// the 256-frame callback into a 200-frame deliverable lap (one legal
/// call) plus a 56-frame remainder (undeliverable). Both halves of the
/// contract are observed in one process.
#[test]
fn loop_200_partial_delivery_no_violations() {
    let mut rig = build_rig(128, 256);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    rig.control
        .request(
            TransportCommand::SetLoop(MediaRange::new(
                MediaTime::new(0, base),
                MediaDuration::new(200, base),
            )),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert!(
        calls.iter().all(|c| (128..=256).contains(&c.frames)),
        "min/max-violating process call detected: {calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c.frames == 56),
        "the 56-frame loop remainder must never reach the plugin"
    );
    let reports = rig.control.drain_reports();
    assert!(
        reports.iter().any(|r| matches!(
            r,
            ruviform_audio_plugin_bridge::NodeReport::UndeliverableSpan { .. }
        )),
        "the 56-frame remainder must be audited: {reports:?}"
    );
}

/// M4-04 — event `frame_offset` is relative to the physical call's
/// start: with `max=128` a 256-frame span splits into two calls; an
/// event at position 200 lands in call 2 at offset 72.
#[test]
fn event_offsets_are_process_start_relative() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(1));
    let ev = |seq| {
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(seq),
            value: ParamValue::new(0.5).unwrap(),
        })
    };
    builder
        .event(MediaTime::new(32, base), 0, ev(0))
        .unwrap()
        .event(MediaTime::new(200, base), 1, ev(1))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.len(), 2, "256 split as 128+128");
    assert_eq!(calls[0].frames, 128);
    assert_eq!(calls[0].offsets, vec![32]);
    assert_eq!(calls[1].frames, 128);
    assert_eq!(
        calls[1].offsets,
        vec![72],
        "200 - call2 start 128 = 72, not the engine offset"
    );
}

/// M4-01 — the capability matrix states honest per-backend levels;
/// L1 stays `Unavailable` where the wire verbs do not exist.
#[test]
fn capability_matrix_is_honest() {
    let l0 = capability_for(Backend::L0);
    assert_eq!(l0.bus_enumeration, CapabilityLevel::Verified);
    assert_eq!(l0.multi_bus, CapabilityLevel::Verified);
    let l1 = capability_for(Backend::L1);
    assert_eq!(l1.bus_enumeration, CapabilityLevel::Unavailable);
}
