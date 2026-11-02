//! Issue #186 evidence — parameter control events through the real
//! bridge: multi-time/multi-target `ParamEvent` order/offset/value
//! delivery, defined rejection, beat→frame conversion, loop
//! refiring, async plan install, dispatch-ledger stage tracking, and
//! the RT (no-alloc) lane.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ruviform_audio::typed::{Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_plugin_bridge::{
    AttachConfig, DispatchStage, DropReason, ParamDispatchLedger, ParamTargetTable,
    PluginNodeFactory, TargetId, TargetResolution, attach,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity, NodeDef,
    NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
};
use ruviform_media_core::{
    BeatPosition, ClockEpochId, DeliveryPolicy, EngineFrame, LoopOccurrence, MediaDuration,
    MediaRange, MediaTime, PlanGeneration, ScheduleBudget, SpanContext, TempoEvent,
    TempoMapBuilder, TempoMapRevision, TimeBase, TransportCommand, TransportGeneration,
    TransportPlanBuilder, TransportSpan, TransportStatus,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, ErasedPlugin, Event,
    EventKind, EventList, HostInput, HostOutput, ParamEvent, ParamFlags, ParamId, ParamInfo,
    ParamValue, ParamWriteMeans, ParamWriteRejection, Plugin, PluginDescriptor, PluginFactory,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
    check_param_write, erase_sendable,
};
use ruviform_plugin_host::{EndpointBounds, HostCommand, HostReply, PluginHost};
use ruviform_rt::RtContext;

const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.param-rec",
    name: "Param Recorder",
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

/// Param table: `1` automatable, `7` automatable+stepped, `9`
/// read-only, `3` no flags.
const PARAMS: [ParamInfo; 4] = [
    ParamInfo {
        id: ParamId(1),
        name: "Gain",
        unit: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: 0.0,
        flags: ParamFlags::AUTOMATABLE,
    },
    ParamInfo {
        id: ParamId(7),
        name: "Step",
        unit: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: 0.0,
        flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
    },
    ParamInfo {
        id: ParamId(9),
        name: "Meter",
        unit: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: 0.0,
        flags: ParamFlags::AUTOMATABLE.union(ParamFlags::READONLY),
    },
    ParamInfo {
        id: ParamId(3),
        name: "Trim",
        unit: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: 0.0,
        flags: ParamFlags::NONE,
    },
];

/// One physical `process` call: `(offset, param, value)` triples as
/// the plugin saw them.
#[derive(Debug)]
struct CallRec {
    frames: usize,
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
        let mut seen = Vec::with_capacity(16);
        for event in events.iter() {
            if let EventKind::ParamValue(pe) = event.kind() {
                seen.push((event.frame_offset(), pe.param_id.0, pe.value.get()));
            }
        }
        self.calls.lock().unwrap().push(CallRec {
            frames: outputs.first().map_or(0, BufferView::frames),
            events: seen,
        });
        for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
            for ch in 0..output.channels() {
                if let (Some(src), Some(dst)) = (input.channel(ch), output.channel_mut(ch)) {
                    dst.copy_from_slice(src);
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

struct Rig {
    _host: PluginHost,
    _runtime: GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    control: ruviform_audio_plugin_bridge::PluginNodeControl,
    calls: Arc<Mutex<Vec<CallRec>>>,
    src_out: PortId,
}

fn fmt() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
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

fn graph_fixture(
    attached: &ruviform_audio_plugin_bridge::Attached,
) -> (
    ruviform_graph::ValidatedGraph,
    PortId,
    ruviform_graph::NodeId,
) {
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
    (g, sp[0], proc)
}

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

    let (g, src_out, node) = graph_fixture(&attached);

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
    factory.insert(node, attached.node);
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
        src_out,
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

/// AC-3 — multiple times × multiple targets: order, frame offset and
/// value arrive exactly through `PluginNode` → `EventList`.
#[test]
fn param_events_multi_time_multi_target_deliver_in_order() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(1));
    builder
        .event(MediaTime::new(64, base), 0, pe(1, 0.5))
        .unwrap()
        .event(MediaTime::new(64, base), 1, pe(7, 0.9))
        .unwrap()
        .event(MediaTime::new(200, base), 2, pe(1, 0.2))
        .unwrap()
        .event(MediaTime::new(32, base), 3, pe(7, 0.1))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.len(), 2, "256 split as 128+128");
    assert_eq!(
        calls[0].events,
        vec![(32, 7, 0.1), (64, 1, 0.5), (64, 7, 0.9)],
        "call [0,128): stable source order at same offset"
    );
    assert_eq!(
        calls[1].events,
        vec![(72, 1, 0.2)],
        "call [128,256): pos 200 lands at offset 72"
    );
}

/// AC-4 — defined rejection for unsupported precision/type/state:
/// read-only, non-automatable, immediate/per-note means and unknown
/// params all reject with typed reasons.
#[test]
fn param_write_defined_rejections() {
    assert_eq!(
        check_param_write(&PARAMS[2], ParamWriteMeans::TimedEvent),
        Err(ParamWriteRejection::ReadOnly),
        "READONLY param must reject timed writes"
    );
    assert_eq!(
        check_param_write(&PARAMS[3], ParamWriteMeans::TimedEvent),
        Err(ParamWriteRejection::NotAutomatable),
        "non-AUTOMATABLE param must reject timed writes"
    );
    for means in [ParamWriteMeans::Immediate, ParamWriteMeans::PerNote] {
        assert_eq!(
            check_param_write(&PARAMS[0], means),
            Err(ParamWriteRejection::UnsupportedMeans),
            "{means:?} has no neutral vocabulary"
        );
    }
    assert_eq!(
        ruviform_plugin_core::check_param_write_for(
            &PARAMS,
            ParamId(42),
            ParamWriteMeans::TimedEvent
        ),
        Err(ParamWriteRejection::UnknownParam),
        "unenumerated ParamId must reject"
    );
    assert_eq!(
        check_param_write(&PARAMS[1], ParamWriteMeans::TimedEvent),
        Ok(()),
        "AUTOMATABLE+STEPPED accepts timed writes (quantization is the consumer's rule)"
    );
}

/// AC-4 — unresolved / defunct / stale-pin targets are defined
/// rejections, never silent misapplication.
#[test]
fn target_binding_rejections() {
    const SHRUNK: [ParamInfo; 1] = [PARAMS[0]];
    let mut table = ParamTargetTable::new();
    let node = rig_node();
    // Unresolved: no binding recorded.
    assert!(matches!(
        table.resolve(TargetId(1)),
        TargetResolution::Unresolved
    ));
    // UnknownParam: binding to a param the plugin does not enumerate.
    assert_eq!(
        table.bind(TargetId(1), node, ParamId(42), &PARAMS),
        Err(ParamWriteRejection::UnknownParam),
    );
    // Resolved + pinned generation.
    let binding = table.bind(TargetId(1), node, ParamId(1), &PARAMS).unwrap();
    assert!(matches!(
        table.resolve(TargetId(1)),
        TargetResolution::Resolved(_)
    ));
    assert_eq!(
        table.resolve_pin(TargetId(1), binding.generation),
        Some(binding)
    );
    // Rebind bumps generation: the old pin is stale → resolve_pin None.
    let rebound = table.bind(TargetId(1), node, ParamId(7), &PARAMS).unwrap();
    assert_ne!(binding.generation, rebound.generation);
    assert_eq!(table.resolve_pin(TargetId(1), binding.generation), None);
    // Defunct: param vanishes from re-enumeration.
    assert!(!table.revalidate(TargetId(1), node, &SHRUNK));
    assert!(matches!(
        table.resolve(TargetId(1)),
        TargetResolution::Defunct
    ));
    // Node detach: every binding on the node goes defunct.
    table.bind(TargetId(2), node, ParamId(1), &PARAMS).unwrap();
    table.invalidate_node(node);
    assert!(matches!(
        table.resolve(TargetId(2)),
        TargetResolution::Defunct
    ));
}

fn rig_node() -> ruviform_graph::NodeId {
    let mut b = GraphBuilder::new();
    let (node, _) = b.add_node(&NodeDef {
        kind: NodeKind::Processor,
        ports: Vec::new(),
    });
    node
}

/// AC-4 — a non-`Processing` plugin is a defined rejection at the
/// attach/process boundary, never silent acceptance.
#[test]
fn non_processing_endpoint_rejected() {
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
    let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
    host.execute(HostCommand::Activate { handle, config })
        .unwrap();
    // Activated but NOT StartProcessing.
    let bounds = EndpointBounds {
        max_frames: 512,
        channels: 2,
    };
    match host.prepare_endpoint(handle, bounds) {
        Err(_) => {} // defined rejection at attach — acceptable
        Ok(mut endpoint) => {
            assert!(
                endpoint
                    .process(&[], &mut [], EventList::empty(0), None)
                    .is_err(),
                "process on a non-Processing endpoint must be a defined rejection"
            );
        }
    }
}

/// AC-5 — `BeatPosition` → `MediaTime` conversion via `TempoMap`
/// (single `NearestEven` rounding) then delivery at the exact frame.
/// At 120 BPM, beat `1/480` = 50 frames @48 kHz; `1/120` = 200 frames.
#[test]
fn beat_to_media_then_delivers_at_exact_frame() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    let mut tm = TempoMapBuilder::new();
    tm.add_tempo_event(TempoEvent::at(BeatPosition::from_int(0), 120_000));
    let map = tm.build().unwrap();

    let pos_a = map
        .beat_to_media(BeatPosition::new(1, 480).unwrap(), base)
        .unwrap();
    let pos_b = map
        .beat_to_media(BeatPosition::new(1, 120).unwrap(), base)
        .unwrap();
    assert_eq!(pos_a, MediaTime::new(50, base));
    assert_eq!(pos_b, MediaTime::new(200, base));

    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(1));
    builder
        .event(pos_a, 0, pe(1, 0.4))
        .unwrap()
        .event(pos_b, 1, pe(7, 0.8))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls[0].events, vec![(50, 1, 0.4)]);
    assert_eq!(calls[1].events, vec![(72, 7, 0.8)]);
}

/// AC-5 — an in-loop event refires exactly once per loop occurrence:
/// `Loop=[0,192)` over 384 frames fires `pos=64` twice, at frames
/// 64 and 256 (offset 64 in their respective calls).
#[test]
fn loop_occurrence_refires_event_once_per_lap() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(1));
    builder
        .event(MediaTime::new(64, base), 0, pe(1, 0.6))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    rig.control
        .request(
            TransportCommand::SetLoop(MediaRange::new(
                MediaTime::new(0, base),
                MediaDuration::new(192, base),
            )),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 384);
    rig.rt.process(384, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    let hits: Vec<&(u32, u32, f64)> = calls.iter().flat_map(|c| c.events.iter()).collect();
    assert_eq!(hits.len(), 2, "one fire per loop occurrence: {hits:?}");
    assert!(
        hits.iter()
            .all(|&&(off, p, v)| off == 64 && p == 1 && v.to_bits() == 0.6f64.to_bits()),
        "each lap delivers the event at call-relative offset 64"
    );
    assert_eq!(calls[1].frames + calls[0].frames, 192);
}

/// AC-5 — an async (control-channel) `install_plan` staged while
/// transport is already playing takes effect at the staged due frame:
/// an event at absolute position 160 lands at offset 32 of the
/// second callback.
#[test]
fn async_install_plan_lands_at_due_frame() {
    let mut rig = build_rig(64, 128);
    let ctx = RtContext::mint();
    let base = TimeBase::sample_rate(48_000).unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &ctx).expect("process");
    assert_eq!(rig.calls.lock().unwrap()[0].events, Vec::new());

    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(2));
    builder
        .event(MediaTime::new(160, base), 0, pe(1, 0.7))
        .unwrap();
    rig.control
        .install_plan(Arc::new(builder.build()), 0)
        .unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &ctx).expect("process");

    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].events,
        vec![(32, 1, 0.7)],
        "plan installed at frame 128 delivers pos-160 event at offset 32"
    );
    let reports = rig.control.drain_reports();
    assert!(
        reports
            .iter()
            .any(|r| matches!(r, ruviform_audio_plugin_bridge::NodeReport::Receipt(_))),
        "install must surface an applied receipt: {reports:?}"
    );
}

/// AC-7 — the dispatch ledger distinguishes `Scheduled → Dispatched →`
/// `Dropped` / `Reported` / `Unavailable` with position + revision.
#[test]
fn ledger_tracks_stage_distinction() {
    let base = TimeBase::sample_rate(48_000).unwrap();
    let node = rig_node();
    let mut table = ParamTargetTable::new();
    table.bind(TargetId(1), node, ParamId(1), &PARAMS).unwrap();
    table.bind(TargetId(2), node, ParamId(7), &PARAMS).unwrap();

    let mut builder = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(1));
    builder
        .event(MediaTime::new(64, base), 0, pe(1, 0.5))
        .unwrap()
        .event(MediaTime::new(160, base), 1, pe(7, 0.9))
        .unwrap();
    let plan = Arc::new(builder.build());

    let mut ledger = ParamDispatchLedger::new();
    assert_eq!(ledger.record_plan(&plan, &table, node), 2);
    let obs = ledger.query(TargetId(1)).unwrap();
    assert!(matches!(obs.stage, DispatchStage::Scheduled));
    assert_eq!(obs.value, Some(ParamValue::new(0.5).unwrap()));
    assert_eq!(obs.position, MediaTime::new(64, base));

    // Deterministic projection: span [0,256) → call [0,128).
    let span = TransportSpan {
        start: EngineFrame::new(ClockEpochId::new(0), 0),
        end: EngineFrame::new(ClockEpochId::new(0), 256),
        ctx: SpanContext {
            status: TransportStatus::Playing,
            position_ticks: 0,
            position_base: base,
            loop_ticks: None,
            occurrence: LoopOccurrence::new(0),
            clock_epoch: ClockEpochId::new(0),
            transport_generation: TransportGeneration::new(0),
            plan_generation: plan.generation,
            tempo_revision: TempoMapRevision::new(0),
        },
        deliverable: true,
    };
    assert_eq!(
        ledger.project_call(&plan, &span, EngineFrame::new(ClockEpochId::new(0), 0), 128),
        1,
        "only the pos-64 event falls in call [0,128)"
    );
    let obs = ledger.query(TargetId(1)).unwrap();
    assert!(matches!(
        obs.stage,
        DispatchStage::Dispatched {
            call_start: 0,
            frame_offset: 64
        }
    ));
    assert!(
        matches!(
            ledger.query(TargetId(2)).unwrap().stage,
            DispatchStage::Scheduled
        ),
        "pos-160 event is not dispatched by call [0,128)"
    );

    // Second call [128,256): pos-160 → offset 32.
    assert_eq!(
        ledger.project_call(
            &plan,
            &span,
            EngineFrame::new(ClockEpochId::new(0), 128),
            128
        ),
        1
    );
    assert!(matches!(
        ledger.query(TargetId(2)).unwrap().stage,
        DispatchStage::Dispatched {
            call_start: 128,
            frame_offset: 32
        }
    ));

    // Scratch overflow on the last projected call drops its entries.
    assert_eq!(
        ledger.apply_report(&ruviform_audio_plugin_bridge::NodeReport::EventScratchOverflow),
        1
    );
    assert!(matches!(
        ledger.query(TargetId(2)).unwrap().stage,
        DispatchStage::Dropped {
            reason: DropReason::EventScratchOverflow
        }
    ));

    // Reported vs Unavailable: target 1 gets a plugin-reported value;
    // closing reporting marks nothing pending for it (dispatched stays
    // reported), but a fresh target with no channel → Unavailable.
    assert!(ledger.record_reported(TargetId(1), ParamValue::new(0.55).unwrap()));
    let obs = ledger.query(TargetId(1)).unwrap();
    assert!(matches!(obs.stage, DispatchStage::Reported));
    assert_eq!(obs.value, Some(ParamValue::new(0.55).unwrap()));

    let mut builder2 = TransportPlanBuilder::<EventKind>::new(base, PlanGeneration::new(2));
    builder2
        .event(MediaTime::new(64, base), 0, pe(1, 0.5))
        .unwrap();
    let plan2 = Arc::new(builder2.build());
    ledger.record_plan(&plan2, &table, node);
    ledger.mark_reporting_unavailable();
    let obs = ledger.query(TargetId(1)).unwrap();
    assert!(
        matches!(obs.stage, DispatchStage::Unavailable),
        "no plugin→host channel → honest Unavailable, got {:?}",
        obs.stage
    );
}

struct Slab<'a>(&'a [f32]);
impl BufferView for Slab<'_> {
    fn channels(&self) -> usize {
        2
    }
    fn frames(&self) -> usize {
        self.0.len()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index < 2).then_some(self.0)
    }
}
struct SlabMut<'a>(&'a mut [f32]);
impl BufferView for SlabMut<'_> {
    fn channels(&self) -> usize {
        2
    }
    fn frames(&self) -> usize {
        self.0.len()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index < 2).then_some(&*self.0)
    }
}
impl BufferViewMut for SlabMut<'_> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        (index < 2).then_some(&mut *self.0)
    }
}

struct CountPlugin {
    events: Arc<AtomicUsize>,
}
impl PluginProcessor for CountPlugin {
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
        self.events.fetch_add(events.len(), Ordering::Relaxed);
        ProcessResult::CONTINUE
    }
}
impl Plugin for CountPlugin {
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
struct CountFactory {
    events: Arc<AtomicUsize>,
}
impl PluginFactory for CountFactory {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }
    fn create(&self) -> Box<dyn ErasedPlugin> {
        ruviform_plugin_core::erase(CountPlugin {
            events: Arc::clone(&self.events),
        })
    }
    fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
        Some(erase_sendable(CountPlugin {
            events: Arc::clone(&self.events),
        }))
    }
}

/// AC-6 (RT lane) — the param dispatch leg (`PluginEndpoint::process`
/// with a populated `EventList`) performs no allocation inside the
/// measured window. The recording uses atomics only, so the plugin
/// itself stays allocation-free.
#[test]
fn rt_param_dispatch_no_alloc() {
    let events = Arc::new(AtomicUsize::new(0));
    let mut host = PluginHost::new();
    host.register_factory(Box::new(CountFactory {
        events: Arc::clone(&events),
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
    host.execute(HostCommand::Activate {
        handle,
        config: ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap(),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    let mut endpoint = host
        .prepare_endpoint(
            handle,
            EndpointBounds {
                max_frames: 512,
                channels: 2,
            },
        )
        .unwrap();

    let evs = [
        Event::new(0, pe(1, 0.5)),
        Event::new(64, pe(7, 0.9)),
        Event::new(100, pe(1, 0.2)),
    ];
    let list = EventList::new(&evs, 128).unwrap();
    let input = [0.0f32; 128];
    let mut output = [0.0f32; 128];
    let transport = TransportInfo::new(
        TransportStatus::Playing,
        MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
    );

    // Warm-up outside the window.
    {
        let ins = [HostInput(&Slab(&input))];
        let mut slab = SlabMut(&mut output);
        let mut outs = [HostOutput(&mut slab)];
        endpoint
            .process(&ins, &mut outs, list, Some(transport))
            .expect("warm process");
    }
    let result = assert_no_alloc::assert_no_alloc(|| {
        let ins = [HostInput(&Slab(&input))];
        let mut slab = SlabMut(&mut output);
        let mut outs = [HostOutput(&mut slab)];
        endpoint
            .process(&ins, &mut outs, list, Some(transport))
            .expect("process")
    });
    assert_eq!(events.load(Ordering::Relaxed), 6);
    let _ = result;
}
