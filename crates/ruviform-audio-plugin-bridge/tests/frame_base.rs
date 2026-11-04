//! Issue #307 evidence — attach-time `frame_base` as the verified
//! non-RT clock context: same-origin accessors on `PluginNodeControl`/
//! `PluginNode`, `check_frame_base` for consumer-side construction,
//! `plan_builder` binding, typed `FrameBaseMismatch` rejection of
//! foreign-base plans (non-destructive), and frame-base/clock-epoch
//! non-conflation.

use std::sync::{Arc, Mutex};

use ruviform_audio::typed::{Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_plugin_bridge::{
    AttachConfig, FrameBaseMismatch, InstallPlanError, PluginNodeFactory, attach,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity, NodeDef,
    NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
};
use ruviform_media_core::{
    DeliveryPolicy, MediaTime, PlanGeneration, ScheduleBudget, TimeBase, TransportCommand,
    TransportPlanBuilder,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, ErasedPlugin, EventKind,
    EventList, ParamEvent, ParamFlags, ParamId, ParamInfo, ParamValue, Plugin, PluginDescriptor,
    PluginFactory, PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult,
    TransportInfo, erase_sendable,
};
use ruviform_plugin_host::{EndpointBounds, HostCommand, HostReply, PluginHost};
use ruviform_rt::RtContext;

const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.frame-base",
    name: "Frame Base Recorder",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

const BUSES: [BusDescriptor; 2] = [
    BusDescriptor {
        direction: BusDirection::Input,
        layout: ChannelLayout::Stereo,
        name: "In",
        role: BusRole::Main,
        enabled: true,
    },
    BusDescriptor {
        direction: BusDirection::Output,
        layout: ChannelLayout::Stereo,
        name: "Out",
        role: BusRole::Main,
        enabled: true,
    },
];

const PARAMS: [ParamInfo; 1] = [ParamInfo {
    id: ParamId(1),
    name: "Gain",
    unit: "",
    min_value: 0.0,
    max_value: 1.0,
    default_value: 0.0,
    flags: ParamFlags::AUTOMATABLE,
}];

fn base_48k() -> TimeBase {
    TimeBase::sample_rate(48_000).unwrap()
}

/// One physical `process` call: `(offset, param, value)` triples as
/// the plugin saw them.
#[derive(Debug)]
struct CallRec {
    events: Vec<(u32, u32, f64)>,
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
        _inputs: &[I],
        _outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let mut seen = Vec::with_capacity(8);
        for event in events.iter() {
            if let EventKind::ParamValue(pe) = event.kind() {
                seen.push((event.frame_offset(), pe.param_id.0, pe.value.get()));
            }
        }
        self.calls.lock().unwrap().push(CallRec { events: seen });
        ProcessResult::CONTINUE
    }
}

impl Plugin for RecPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }
    fn params(&self) -> &[ParamInfo] {
        &PARAMS
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

fn fmt() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
}

/// Drives a `RecPlugin` fixture to `Processing` and attaches with
/// `frame_base` — no graph wiring, for accessor-level assertions.
fn attach_only(
    frame_base: TimeBase,
    min_frames: u32,
    max_frames: u32,
) -> (
    PluginHost,
    ruviform_audio_plugin_bridge::Attached,
    Arc<Mutex<Vec<CallRec>>>,
) {
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
            frame_base,
            policy: DeliveryPolicy::Exact,
            budget: ScheduleBudget::default(),
            input: None,
        },
    )
    .expect("attach");
    (host, attached, calls)
}

struct Rig {
    _host: PluginHost,
    _runtime: GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    control: ruviform_audio_plugin_bridge::PluginNodeControl,
    calls: Arc<Mutex<Vec<CallRec>>>,
    src_out: PortId,
}

fn port(direction: Direction, format: PortFormat) -> PortDef {
    PortDef {
        direction,
        role: ruviform_graph::PortRole::Main,
        format,
        multiplicity: Multiplicity::unbounded(0),
        capabilities: Vec::new(),
        availability: Availability::Always,
        activation: Activation::Enabled,
    }
}

/// Full graph rig: `Source → plugin → Sink` compiled into an
/// `RtRuntime`, for process-level delivery assertions.
fn build_rig(min_frames: u32, max_frames: u32) -> Rig {
    let (host, attached, calls) = attach_only(base_48k(), min_frames, max_frames);

    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&NodeDef {
        kind: NodeKind::Source,
        ports: vec![port(Direction::Output, PortFormat::Audio(fmt()))],
    });
    let (proc, pp) = b.add_node(&NodeDef {
        kind: NodeKind::Processor,
        ports: attached.ports.clone(),
    });
    let (sink, kp) = b.add_node(&NodeDef {
        kind: NodeKind::Sink,
        ports: vec![port(Direction::Input, PortFormat::Audio(fmt()))],
    });
    for (from_node, from_port, to_node, to_port) in
        [(src, sp[0], proc, pp[0]), (proc, pp[1], sink, kp[0])]
    {
        b.connect(&ConnDef {
            from: PortRef {
                node: from_node,
                port: from_port,
            },
            to: PortRef {
                node: to_node,
                port: to_port,
            },
            delay: EdgeDelay::Zero,
            channel_map: None,
        });
    }
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
        src_out: sp[0],
    }
}

fn feed(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId, frames: usize) {
    let zeros = vec![0.0f32; frames];
    let mut slot = rt.input_writer(port, frames).expect("input bound");
    for ch in 0..2 {
        let plane = slot.data.channel_mut(ch).expect("channel");
        plane[..frames].copy_from_slice(&zeros);
    }
}

fn pe(id: u32, value: f64) -> EventKind {
    EventKind::ParamValue(ParamEvent {
        param_id: ParamId(id),
        value: ParamValue::new(value).unwrap(),
    })
}

/// AC-1 — the attach-time `TimeBase` round-trips through both public
/// halves: `control.frame_base() == node.frame_base() ==
/// AttachConfig.frame_base` (single origin). Substantive equality
/// means an unnormalized-but-equal `TimeBase` also compares equal.
#[test]
fn verified_context_round_trips_same_origin() {
    let (_host, attached, _calls) = attach_only(base_48k(), 1, 512);
    assert_eq!(attached.control.frame_base(), base_48k());
    assert_eq!(attached.node.frame_base(), base_48k());
    assert_eq!(
        attached.control.frame_base(),
        attached.node.frame_base(),
        "control and node contexts share the AttachConfig origin"
    );
    // 2/96000 normalizes to 1/48000 — substantive equivalence, not
    // raw-pair equality.
    assert_eq!(
        attached.control.frame_base(),
        TimeBase::new(2, 96_000).unwrap()
    );
}

/// AC-3 — `check_frame_base` is the typed dispatcher-construction
/// gate: equal bases pass (including normalized-equal forms), foreign
/// bases fail with `expected`/`actual` payload.
#[test]
fn check_frame_base_accepts_and_rejects() {
    let (_host, attached, _calls) = attach_only(base_48k(), 1, 512);
    let control = &attached.control;
    control.check_frame_base(base_48k()).unwrap();
    control
        .check_frame_base(TimeBase::new(2, 96_000).unwrap())
        .unwrap();
    let foreign = TimeBase::new(1, 44_100).unwrap();
    assert_eq!(
        control.check_frame_base(foreign),
        Err(FrameBaseMismatch {
            expected: base_48k(),
            actual: foreign,
        }),
        "foreign base must carry the typed payload a consumer ctor can `?`-propagate"
    );
}

/// AC-2 — `plan_builder` binds the verified base; a hand-built plan in
/// a foreign base is rejected before staging and is non-destructive:
/// nothing queued, no reports, no overflow bump, and a subsequent
/// correct install applies normally.
#[test]
fn foreign_base_plan_install_rejected_nondestructively() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();

    // The verified-context builder produces plans that always pass.
    let mut good = rig.control.plan_builder(PlanGeneration::new(1));
    good.event(MediaTime::new(64, rig.control.frame_base()), 0, pe(1, 0.5))
        .unwrap();
    let good_plan = Arc::new(good.build());
    assert_eq!(good_plan.frame_base, rig.control.frame_base());

    // A plan compiled in a foreign base never reaches the node.
    let foreign = TimeBase::new(1, 96_000).unwrap();
    let mut bad = TransportPlanBuilder::<EventKind>::new(foreign, PlanGeneration::new(2));
    bad.event(MediaTime::new(64, foreign), 0, pe(1, 0.9))
        .unwrap();
    assert_eq!(
        rig.control.install_plan(Arc::new(bad.build()), 0),
        Err(InstallPlanError::FrameBaseMismatch(FrameBaseMismatch {
            expected: base_48k(),
            actual: foreign,
        })),
        "foreign-base plan must be typed-rejected, never staged"
    );
    assert_eq!(
        rig.control.snapshot().control_overflows,
        0,
        "rejection happens before the queue — no drop is counted"
    );
    assert!(
        rig.control.drain_reports().is_empty(),
        "a rejected plan emits no report and mutates nothing"
    );

    // The correct plan still installs and delivers — old state intact.
    rig.control.install_plan(good_plan, 0).unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    let hits: Vec<&(u32, u32, f64)> = calls.iter().flat_map(|c| c.events.iter()).collect();
    assert_eq!(
        hits.as_slice(),
        &[&(64, 1, 0.5)],
        "the verified-base plan delivers at absolute frame 64"
    );
}

/// AC-1 — a `TimedParamEvent` assembled against `control.frame_base()`
/// (the consumer-side verified context) delivers at the exact absolute
/// frame through the real RT path.
#[test]
fn verified_base_timed_param_event_lands_at_absolute_frame() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = rig.control.frame_base();
    let mut builder = rig.control.plan_builder(PlanGeneration::new(1));
    builder
        .event(MediaTime::new(32, base), 0, pe(1, 0.25))
        .unwrap()
        .event(MediaTime::new(200, base), 1, pe(1, 0.75))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls[0].events, vec![(32, 1, 0.25)]);
    assert_eq!(
        calls[1].events,
        vec![(72, 1, 0.75)],
        "pos 200 - call2 start 128 = offset 72"
    );
}

/// AC-4 — `device_restart` rebases the clock epoch, not the frame
/// base: `frame_base()` stays the attach value across the epoch bump
/// (identities are never conflated).
#[test]
fn device_restart_rebases_epoch_not_frame_base() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    rig.control.device_restart(0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &ctx).expect("process");

    assert_eq!(rig.control.frame_base(), base_48k());
    let mut builder = rig.control.plan_builder(PlanGeneration::new(1));
    builder
        .event(MediaTime::new(64, rig.control.frame_base()), 0, pe(1, 0.4))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    let hits: Vec<&(u32, u32, f64)> = calls.iter().flat_map(|c| c.events.iter()).collect();
    assert_eq!(
        hits.as_slice(),
        &[&(64, 1, 0.4)],
        "post-restart the verified base still delivers at absolute frame 64"
    );
}

/// AC-2 — a full control queue still reports the unchanged `QueueFull`
/// semantics through the new error type.
#[test]
fn install_plan_queue_full_still_typed() {
    let mut rig = build_rig(64, 128);
    while rig.control.request(TransportCommand::Play, 0).is_ok() {}
    assert!(
        rig.control.snapshot().control_overflows > 0,
        "the drain loop must have saturated the bounded queue"
    );
    let mut builder = rig.control.plan_builder(PlanGeneration::new(1));
    builder
        .event(MediaTime::new(0, rig.control.frame_base()), 0, pe(1, 0.5))
        .unwrap();
    assert_eq!(
        rig.control.install_plan(Arc::new(builder.build()), 0),
        Err(InstallPlanError::QueueFull(
            ruviform_audio_plugin_bridge::QueueFull
        )),
        "queue-full keeps its typed shape through InstallPlanError"
    );
}
