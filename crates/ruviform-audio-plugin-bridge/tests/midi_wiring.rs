//! Issue #188 — the wired path: `MidiInput` → `InputTx::pump` →
//! `InputStage` → `CallSink` merge → plugin `EventList`, plus
//! recorded-plan replay, loop refire and seek `release_all`+chase
//! through the real `PluginNode`/`GraphRuntime`.
//!
//! Covers: live events arriving with block-relative frame offsets;
//! plan MIDI replay at scheduled positions; per-lap refire at loop
//! boundaries; transport discontinuity releasing delivered notes and
//! re-firing notes still sounding at the seek target; defined
//! overflow reporting; and the RT no-allocation lane.

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use ruviform_audio::typed::{Compiler, CompilerConfig, DEFAULT_ALIGNMENT, GraphRuntime};
use ruviform_audio_core::device::{DeviceError, DeviceResult};
use ruviform_audio_core::midi_input::mock::MockBackend;
use ruviform_audio_core::midi_input::{
    MidiInput, MidiInputEnumerator, MidiInputInfo, MidiPortCapabilities, MidiPortOrigin,
    MidiReadStats, TimedMidiInput,
};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_audio_plugin_bridge::{
    AttachConfig, AttachError, InputConfig, InputTx, NodeReport, PluginNodeFactory, attach,
};
use ruviform_graph::{
    Activation, Availability, ConnDef, Direction, EdgeDelay, GraphBuilder, Multiplicity, NodeDef,
    NodeKind, PortDef, PortFormat, PortId, PortRef, Validator,
};
use ruviform_media_core::{
    ClockAnchor, ClockDomainId, ClockEpochId, DeliveryPolicy, EngineFrame, InputStamp,
    MediaDuration, MediaRange, MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup,
    MidiKind, MidiValue, NoteKey, PlanGeneration, ScheduleBudget, TimeBase, TimestampProvenance,
    TransportCommand, TransportPlanBuilder,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, ErasedPlugin, EventKind,
    EventList, ParamInfo, Plugin, PluginDescriptor, PluginFactory, PluginProcessor, PrepareError,
    ProcessConfig, ProcessContext, ProcessResult, TransportInfo, erase_sendable,
};
use ruviform_plugin_host::{EndpointBounds, HostCommand, HostReply, PluginHost};
use ruviform_rt::RtContext;

const DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.midi-wiring",
    name: "MIDI Wiring Recorder",
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

// ---------------------------------------------------------------------------
// event vocabulary
// ---------------------------------------------------------------------------

fn note_on(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(100),
        },
    }
}

fn note_on_ch(channel: u8, key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(channel).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(100),
        },
    }
}

fn note_off(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOff {
            key: NoteKey::new(key).unwrap(),
            velocity: 0,
        },
    }
}

fn cc(ctrl: u8, v: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::ControlChange {
            controller: MidiController::new(ctrl).unwrap(),
            value: MidiValue::from_u7(v),
        },
    }
}

fn me(midi: MidiEvent) -> EventKind {
    EventKind::Midi(midi)
}

// ---------------------------------------------------------------------------
// timed input fixture
// ---------------------------------------------------------------------------

const STAMP_DOMAIN: ClockDomainId = ClockDomainId::new(7);

fn stamp_base() -> TimeBase {
    TimeBase::new(1, 1_000_000).unwrap() // µs source clock
}

fn anchor() -> ClockAnchor {
    // stamp 0 ↔ engine frame 0 of the node's startup epoch.
    ClockAnchor {
        domain: STAMP_DOMAIN,
        stamp_ticks: 0,
        stamp_base: stamp_base(),
        frame: EngineFrame::new(ClockEpochId::new(0), 0),
        calibration: TimestampProvenance::Carried,
    }
}

fn timed_us(us: u64, event: MidiEvent) -> TimedMidiInput {
    TimedMidiInput {
        stamp: InputStamp {
            ticks: us,
            base: stamp_base(),
            domain: STAMP_DOMAIN,
            provenance: TimestampProvenance::Carried,
        },
        event,
    }
}

struct ScriptedInput {
    pending: VecDeque<TimedMidiInput>,
    dropped: u64,
}

impl ScriptedInput {
    fn of(events: Vec<TimedMidiInput>) -> Self {
        Self {
            pending: VecDeque::from(events),
            dropped: 0,
        }
    }
}

impl MidiInput for ScriptedInput {
    fn read(&mut self, out: &mut Vec<TimedMidiInput>, max: usize) -> DeviceResult<MidiReadStats> {
        let n = self.pending.len().min(max);
        out.extend(self.pending.drain(..n));
        let stats = MidiReadStats::new(self.dropped);
        self.dropped = 0;
        Ok(stats)
    }
}

// ---------------------------------------------------------------------------
// recording plugin
// ---------------------------------------------------------------------------

/// The event subset the assertions care about.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Ev {
    On(u8),
    Off(u8),
    Cc(u8),
    Param(u32),
    Other,
}

fn classify(e: &ruviform_plugin_core::Event) -> Ev {
    match e.kind() {
        EventKind::Midi(m) => match m.kind {
            MidiKind::NoteOn { key, .. } if m.effective_note_state() == Some(true) => {
                Ev::On(key.get())
            }
            MidiKind::NoteOn { key, .. } | MidiKind::NoteOff { key, .. } => Ev::Off(key.get()),
            MidiKind::ControlChange { controller, .. } => Ev::Cc(controller.get()),
            _ => Ev::Other,
        },
        EventKind::ParamValue(p) => Ev::Param(p.param_id.0),
        _ => Ev::Other,
    }
}

/// Flat, pre-reserved call recording: `marks[i]` is the index into
/// `events` where call `i`'s events begin. Reserving both vectors at
/// build time keeps `process` allocation-free on the RT lane.
#[derive(Debug, Default)]
struct Recording {
    marks: Vec<usize>,
    events: Vec<(u32, Ev)>,
}

impl Recording {
    fn new() -> Self {
        Self {
            marks: Vec::with_capacity(256),
            events: Vec::with_capacity(8_192),
        }
    }
}

struct RecPlugin {
    rec: Arc<Mutex<Recording>>,
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
        let mut rec = self.rec.lock().unwrap();
        let mark = rec.events.len();
        rec.marks.push(mark);
        rec.events
            .extend(events.iter().map(|e| (e.frame_offset(), classify(&e))));
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
    rec: Arc<Mutex<Recording>>,
}

impl PluginFactory for RecFactory {
    fn descriptor(&self) -> PluginDescriptor {
        DESCRIPTOR
    }
    fn create(&self) -> Box<dyn ErasedPlugin> {
        ruviform_plugin_core::erase(RecPlugin {
            rec: Arc::clone(&self.rec),
        })
    }
    fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
        Some(erase_sendable(RecPlugin {
            rec: Arc::clone(&self.rec),
        }))
    }
}

// ---------------------------------------------------------------------------
// rig: Source → plugin → Sink through the real typed runtime
// ---------------------------------------------------------------------------

struct Rig {
    _host: PluginHost,
    _runtime: GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    control: ruviform_audio_plugin_bridge::PluginNodeControl,
    input_tx: Option<InputTx>,
    rec: Arc<Mutex<Recording>>,
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

fn attach_config(max_frames: u32, input: Option<InputConfig>) -> AttachConfig {
    AttachConfig {
        buses: BUSES.to_vec(),
        port_format: fmt(),
        bounds: EndpointBounds {
            max_frames,
            channels: 2,
        },
        process_config: ProcessConfig::new(SampleRate::new(48_000).unwrap(), 64, max_frames)
            .unwrap(),
        frame_base: TimeBase::sample_rate(48_000).unwrap(),
        policy: DeliveryPolicy::Exact,
        budget: ScheduleBudget::default(),
        input,
    }
}

fn build_rig(max_frames: u32, with_input: bool) -> Rig {
    let rec = Arc::new(Mutex::new(Recording::new()));
    let mut host = PluginHost::new();
    host.register_factory(Box::new(RecFactory {
        rec: Arc::clone(&rec),
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
    let cfg = attach_config(max_frames, with_input.then_some(InputConfig::new(64)));
    host.execute(HostCommand::Activate {
        handle,
        config: cfg.process_config,
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    let attached = attach(&mut host, handle, &cfg).expect("attach");

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
        input_tx: attached.input_tx,
        rec,
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

fn base() -> TimeBase {
    TimeBase::sample_rate(48_000).unwrap()
}

fn plan_with(
    entries: &[(u64, u64, EventKind)],
) -> Arc<ruviform_media_core::TransportPlan<EventKind>> {
    let mut b = TransportPlanBuilder::<EventKind>::new(base(), PlanGeneration::new(1));
    for &(pos, seq, k) in entries {
        b.event(MediaTime::new(i64::try_from(pos).unwrap(), base()), seq, k)
            .unwrap();
    }
    Arc::new(b.build())
}

fn events_of(rec: &Arc<Mutex<Recording>>) -> Vec<Vec<(u32, Ev)>> {
    let rec = rec.lock().unwrap();
    (0..rec.marks.len())
        .map(|i| {
            let end = rec.marks.get(i + 1).copied().unwrap_or(rec.events.len());
            rec.events[rec.marks[i]..end].to_vec()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// live input → plugin wiring
// ---------------------------------------------------------------------------

/// The opt-in leg exists only when `AttachConfig::input` is set.
#[test]
fn input_leg_is_opt_in() {
    let rig = build_rig(128, true);
    assert!(rig.input_tx.is_some());
    let rig = build_rig(128, false);
    assert!(rig.input_tx.is_none());
}

/// A `channel_capacity` of `0` is a defined attach rejection.
#[test]
fn input_leg_rejects_zero_capacity() {
    let calls = Arc::new(Mutex::new(Recording::new()));
    let mut host = PluginHost::new();
    host.register_factory(Box::new(RecFactory { rec: calls }));
    let HostReply::Instantiated { handle } = host
        .execute(HostCommand::Instantiate {
            plugin_id: DESCRIPTOR.id.to_owned(),
        })
        .unwrap()
    else {
        panic!("Instantiate")
    };
    host.execute(HostCommand::Initialize { handle }).unwrap();
    let cfg = attach_config(128, Some(InputConfig::new(0)));
    host.execute(HostCommand::Activate {
        handle,
        config: cfg.process_config,
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    assert!(matches!(
        attach(&mut host, handle, &cfg),
        Err(AttachError::InputConfig)
    ));
}

/// µs-stamped backend events reach the plugin through the real
/// callback at their mapped call-relative frame offsets, split across
/// physical calls at the max-frames boundary.
#[test]
fn live_input_reaches_plugin_with_frame_offsets() {
    let mut rig = build_rig(128, true);
    rig.control.set_clock_anchor(Some(anchor())).unwrap();
    let mut input = ScriptedInput::of(vec![
        timed_us(0, note_on(60)),      // frame 0
        timed_us(2_000, note_off(60)), // frame 96
        timed_us(4_000, note_on(62)),  // frame 192 → call 2, offset 64
        timed_us(4_000, cc(64, 127)),  // frame 192 — same tick, kept order
    ]);
    let stats = rig.input_tx.as_mut().unwrap().pump(&mut input).unwrap();
    assert_eq!(stats.pushed, 4);

    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    assert_eq!(calls.len(), 2, "256 frames split as 128+128");
    assert_eq!(
        calls[0],
        vec![(0, Ev::On(60)), (96, Ev::Off(60))],
        "call [0,128): stamp→frame offsets preserved"
    );
    assert_eq!(
        calls[1],
        vec![(64, Ev::On(62)), (64, Ev::Cc(64))],
        "call [128,256): pos 192 lands at call-relative offset 64, same-tick order kept"
    );
}

/// Without a published anchor nothing maps — events stay pending
/// rather than being delivered blind; once the anchor lands the same
/// events map normally.
#[test]
fn live_input_waits_for_clock_anchor() {
    let mut rig = build_rig(128, true);
    let mut input = ScriptedInput::of(vec![timed_us(0, note_on(60))]);
    rig.input_tx.as_mut().unwrap().pump(&mut input).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert!(
        events_of(&rig.rec).iter().all(Vec::is_empty),
        "no anchor → nothing delivered"
    );

    rig.control.set_clock_anchor(Some(anchor())).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    let calls = events_of(&rig.rec);
    // Stamp 0 mapped to frame 0 — now late → clamped to offset 0 of
    // the next block (defined late-delivery degradation, never dropped).
    assert_eq!(calls[1], vec![(0, Ev::On(60))]);
}

/// Issue #253 — the deterministic scripted backend
/// (`midi_input::mock::MockBackend`) drives the identical
/// `InputTx::pump` → `InputStage` → plugin path a real OS adapter
/// would: enumeration, `open`, timestamped injection, bounded drain,
/// backend drop accounting and `close` all exercise the port
/// contract rather than a test-only shortcut.
#[test]
fn mock_backend_drives_input_stage_to_plugin() {
    let mut rig = build_rig(128, true);
    rig.control.set_clock_anchor(Some(anchor())).unwrap();

    let mut backend = MockBackend::new();
    let port = backend.add_port(MidiInputInfo::new(
        "mock-0".to_owned(),
        "Mock Port 0".to_owned(),
        MidiPortOrigin::External,
        MidiPortCapabilities::stamped(false, STAMP_DOMAIN),
    ));
    port.push_batch([
        timed_us(0, note_on(60)),
        timed_us(2_000, note_off(60)),
        timed_us(4_000, note_on(64)),
    ]);
    port.inject_dropped(2);
    port.inject_unsupported(4);

    // Enumerate → `info` → `open` through the port traits — the same
    // calls an engine-side adapter selection makes.
    let ports = backend.input_ports().unwrap();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].info().id, "mock-0");
    assert_eq!(ports[0].info().name, "Mock Port 0");
    assert!(ports[0].info().capabilities.provides_timestamps);
    let mut input = ports[0].open().unwrap();
    assert!(port.is_open());

    // Scripted backend drops and untranslatable traffic surface
    // through `pump`'s stats and the `InputTx` cumulative counters —
    // `unsupported` must not die at the port boundary (Issue #253).
    let stats = rig.input_tx.as_mut().unwrap().pump(&mut *input).unwrap();
    assert_eq!(stats.received, 3);
    assert_eq!(stats.pushed, 3);
    assert_eq!(stats.backend_dropped, 2);
    assert_eq!(stats.backend_unsupported, 4);
    assert_eq!(rig.input_tx.as_ref().unwrap().dropped_backend(), 2);
    assert_eq!(rig.input_tx.as_ref().unwrap().unsupported_backend(), 4);

    // `unsupported` accumulates across pumps like `dropped` does —
    // a second scripted batch adds to the cumulative counter.
    port.inject_unsupported(3);
    let stats = rig.input_tx.as_mut().unwrap().pump(&mut *input).unwrap();
    assert_eq!(stats.backend_unsupported, 3);
    assert_eq!(stats.backend_dropped, 0);
    assert_eq!(rig.input_tx.as_ref().unwrap().unsupported_backend(), 7);
    assert_eq!(rig.input_tx.as_ref().unwrap().dropped_backend(), 2);

    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    assert_eq!(calls.len(), 2, "256 frames split as 128+128");
    assert_eq!(calls[0], vec![(0, Ev::On(60)), (96, Ev::Off(60))]);
    assert_eq!(calls[1], vec![(64, Ev::On(64))]);

    // The channel path's `unsupported` counter lives on `InputTx`
    // (the `MidiInput::read` caller), not on the stage — the stage's
    // `unsupported_backend` only accumulates on its own direct-`read`
    // path, so the status mirror stays zero here (same ownership
    // split as `dropped_backend`).
    assert_eq!(
        rig.control.snapshot().input.unsupported_backend,
        0,
        "channel-path unsupported is an InputTx counter, not a stage counter"
    );

    // `close` invalidates the subscription — a further pump reports
    // the backend error instead of delivering stale events.
    input.close();
    assert!(port.is_closed());
    port.push(timed_us(0, note_on(62)));
    assert!(matches!(
        rig.input_tx.as_mut().unwrap().pump(&mut *input),
        Err(DeviceError::StreamInvalidated)
    ));
}

// ---------------------------------------------------------------------------
// recorded plan replay / loop / seek chase
// ---------------------------------------------------------------------------

/// Plan MIDI events replay through the scheduler at their scheduled
/// positions with call-relative offsets.
#[test]
fn plan_midi_replays_at_scheduled_offsets() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(
            plan_with(&[
                (64, 0, me(note_on(60))),
                (96, 1, me(cc(1, 64))),
                (192, 2, me(note_off(60))),
            ]),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    assert_eq!(
        calls[0],
        vec![(64, Ev::On(60)), (96, Ev::Cc(1))],
        "call [0,128)"
    );
    assert_eq!(
        calls[1],
        vec![(64, Ev::Off(60))],
        "pos 192 → call-relative 64"
    );
}

/// Loop `[0,192)` over 384 frames refires the in-loop events once per
/// lap at their in-lap positions.
#[test]
fn loop_refires_midi_events_per_lap() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(
            plan_with(&[(64, 0, me(note_on(60))), (128, 1, me(note_off(60)))]),
            0,
        )
        .unwrap();
    rig.control
        .request(
            TransportCommand::SetLoop(MediaRange::new(
                MediaTime::new(0, base()),
                MediaDuration::new(192, base()),
            )),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 384);
    rig.rt.process(384, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    // Call partitioning is balanced: each 192-frame lap splits as
    // 96+96, so the pos-128 off lands at offset 32 of each lap's
    // second call. The lap-2 occurrence bump is a discontinuity but
    // the ledger is empty — nothing releases, nothing chases.
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[0], vec![(64, Ev::On(60))]);
    assert_eq!(calls[1], vec![(32, Ev::Off(60))]);
    assert_eq!(calls[2], vec![(64, Ev::On(60))], "lap 2 refires pos 64");
    assert_eq!(calls[3], vec![(32, Ev::Off(60))], "lap 2 refires pos 128");
}

/// Seeking lands mid-note: the discontinuity release tears down every
/// delivered note still in the ledger (offset 0, ahead of everything),
/// chase re-fires the key still sounding at the target position, and
/// plan events ahead of the position deliver normally in-window.
#[test]
fn seek_releases_and_chases_sounding_notes() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(
            plan_with(&[
                (0, 0, me(note_on(60))), // sounding until... never
                (10, 1, me(note_on(61))),
                (30, 2, me(note_off(61))), // key 61 completes early
                (80, 3, me(note_on(64))),  // starts after the seek target
            ]),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(
        events_of(&rig.rec)[0],
        vec![
            (0, Ev::On(60)),
            (10, Ev::On(61)),
            (30, Ev::Off(61)),
            (80, Ev::On(64))
        ]
    );
    // Ledger now holds keys 60 and 64.

    rig.control
        .request(TransportCommand::Seek(MediaTime::new(64, base())), 0)
        .unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    // Call after the seek (position [64,192)):
    // - release: off 60, off 64 at offset 0 (ledger insertion order)
    // - chase: key 60 still sounding at pos 64 → re-fired at offset 0
    // - plan: pos 80 lies inside the new window → on 64 at offset 16
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1],
        vec![
            (0, Ev::Off(60)),
            (0, Ev::Off(64)),
            (0, Ev::On(60)),
            (16, Ev::On(64)),
        ],
        "release before chase before in-window events"
    );
}

/// `Stop` is a discontinuity too: delivered notes are released at
/// offset 0 and nothing chases (transport not playing). Live input
/// keeps flowing while stopped.
#[test]
fn stop_releases_sounding_notes() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(plan_with(&[(0, 0, me(note_on(60)))]), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(events_of(&rig.rec)[0], vec![(0, Ev::On(60))]);

    rig.control.request(TransportCommand::Stop, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    let calls = events_of(&rig.rec);
    assert_eq!(
        calls[1],
        vec![(0, Ev::Off(60))],
        "stop releases the ledger's sounding key at offset 0"
    );
}

/// Live notes ride the same ledger: a live `NoteOn` delivered while
/// playing is released when the transport stops — no stuck notes from
/// the input leg either.
#[test]
fn stop_releases_live_notes_too() {
    let mut rig = build_rig(128, true);
    rig.control.set_clock_anchor(Some(anchor())).unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    let mut input = ScriptedInput::of(vec![timed_us(0, note_on(60))]);
    rig.input_tx.as_mut().unwrap().pump(&mut input).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(events_of(&rig.rec)[0], vec![(0, Ev::On(60))]);

    rig.control.request(TransportCommand::Stop, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(
        events_of(&rig.rec)[1],
        vec![(0, Ev::Off(60))],
        "the merged delivery fed the ledger — stop releases live notes"
    );
}

/// Defined-loss reporting: a plan with more sounding keys than the
/// bounded chase table (and more than the delivered-note ledger)
/// surfaces `LedgerOverflow` during delivery and `ChaseOverflow` at
/// the discontinuity — never a silent drop.
#[test]
fn overflow_reports_are_observable() {
    let mut rig = build_rig(128, false);
    // 130 distinct sounding keys via 2 channels × 65 keys.
    let entries: Vec<(u64, u64, EventKind)> = (0u8..130)
        .map(|i| (u64::from(i), u64::from(i), me(note_on_ch(i / 65, i % 65))))
        .collect();
    rig.control.install_plan(plan_with(&entries), 0).unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &RtContext::mint()).expect("process");

    // 130 note-ons delivered into the 128-entry delivered-note ledger.
    let reports = rig.control.drain_reports();
    assert!(
        reports
            .iter()
            .any(|r| matches!(r, NodeReport::LedgerOverflow)),
        "ledger force-release must be reported: {reports:?}"
    );

    // Seek past every prefix on: release ≤128 + chase scan over all
    // 130 sounding keys — the 128-entry chase table evicts and
    // reports.
    rig.control
        .request(TransportCommand::Seek(MediaTime::new(200, base())), 0)
        .unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    let reports = rig.control.drain_reports();
    assert!(
        reports
            .iter()
            .any(|r| matches!(r, NodeReport::ChaseOverflow { dropped } if *dropped > 0)),
        "chase table bound must report its loss: {reports:?}"
    );
}

// ---------------------------------------------------------------------------
// RT lane
// ---------------------------------------------------------------------------

/// The full wired callback — pump drain, collect, release/chase,
/// merge, ledger feed, dispatch — performs no allocation on the RT
/// path once warmed.
#[test]
#[cfg(debug_assertions)]
fn rt_midi_wiring_no_alloc() {
    let mut rig = build_rig(128, true);
    rig.control.set_clock_anchor(Some(anchor())).unwrap();
    rig.control
        .install_plan(
            plan_with(&[(0, 0, me(note_on(60))), (192, 1, me(note_off(60)))]),
            0,
        )
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    let mut input = ScriptedInput::of(vec![timed_us(1_000, note_on(64))]);
    rig.input_tx.as_mut().unwrap().pump(&mut input).unwrap();

    // Warm-up callback (first-touch state settles outside the window).
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("warm");

    let mut input2 = ScriptedInput::of(vec![timed_us(5_000, note_off(64))]);
    rig.input_tx.as_mut().unwrap().pump(&mut input2).unwrap();
    feed(&mut rig.rt, rig.src_out, 256);
    let ctx = RtContext::mint();
    assert_no_alloc::assert_no_alloc(|| {
        rig.rt.process(256, &ctx).expect("process");
    });

    let calls = events_of(&rig.rec);
    // Warm call [0,128): plan on@0 + live on@48. Measured callback's
    // call [128,256): plan off@192 → offset 64, live off (frame 240)
    // → offset 112.
    assert_eq!(
        calls[1],
        vec![(64, Ev::Off(60)), (112, Ev::Off(64))],
        "plan + live merge inside the no-alloc window: {calls:?}"
    );
}

// ---------------------------------------------------------------------------
// review-fix regressions
// ---------------------------------------------------------------------------

/// A same-epoch `SetClock` rebases the frame cursor — position is
/// frame-derived while playing, so the jump is a real transport
/// discontinuity the ctx key alone cannot see: the delivered-note
/// ledger must release and the plan prefix must chase.
#[test]
fn set_clock_jump_triggers_release_and_chase() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(plan_with(&[(0, 0, me(note_on(60)))]), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(events_of(&rig.rec)[0], vec![(0, Ev::On(60))]);

    // Same-epoch cursor jump: derived position leaps ~500 frames past
    // the expected successor — a hidden discontinuity without the
    // position-continuity check.
    rig.control.set_clock(ClockEpochId::new(0), 628).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    let calls = events_of(&rig.rec);
    assert_eq!(
        calls[1],
        vec![(0, Ev::Off(60)), (0, Ev::On(60))],
        "position jump = discontinuity: release then chase at offset 0"
    );
}

/// A `SetClock` naming a foreign epoch is ignored — accepting it
/// would make every callback boundary unrepresentable (a silent
/// output stall). Epoch rebases go through `DeviceRestart`.
#[test]
fn foreign_epoch_set_clock_is_rejected() {
    let mut rig = build_rig(128, false);
    rig.control
        .install_plan(plan_with(&[(64, 0, me(note_on(60)))]), 0)
        .unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    assert_eq!(events_of(&rig.rec)[0], vec![(64, Ev::On(60))]);

    rig.control.set_clock(ClockEpochId::new(9), 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 128);
    rig.rt.process(128, &RtContext::mint()).expect("process");
    let reports = rig.control.drain_reports();
    assert!(
        reports
            .iter()
            .any(|r| matches!(r, NodeReport::StaleEpochControl)),
        "foreign-epoch SetClock must be reported: {reports:?}"
    );
    assert_eq!(rig.control.status().snapshot().stale_epoch_controls, 1);
}

/// Capacity eviction from the delivered-note ledger owes the plugin
/// a synthetic Note Off — drained at offset 0 of the next call so an
/// evicted key can never ring forever.
#[test]
fn ledger_eviction_delivers_synthetic_off() {
    let mut rig = build_rig(128, false);
    // 130 distinct sounding keys via 2 channels × 65 keys — the
    // 128-entry ledger force-releases the two oldest on delivery.
    let entries: Vec<(u64, u64, EventKind)> = (0u8..130)
        .map(|i| (u64::from(i), u64::from(i), me(note_on_ch(i / 65, i % 65))))
        .collect();
    rig.control.install_plan(plan_with(&entries), 0).unwrap();
    rig.control.request(TransportCommand::Play, 0).unwrap();
    feed(&mut rig.rt, rig.src_out, 384);
    rig.rt.process(384, &RtContext::mint()).expect("process");

    let calls = events_of(&rig.rec);
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].len(), 128, "call 0: 128 note-ons");
    assert_eq!(calls[1].len(), 2, "call 1: 2 note-ons, evicting k0/k1");
    assert_eq!(
        calls[2],
        vec![(0, Ev::Off(0)), (0, Ev::Off(1))],
        "evicted keys' synthetic offs at offset 0 of the next call"
    );
}

// ---------------------------------------------------------------------------
// real OS backend opt-in lane (Issue #253)
// ---------------------------------------------------------------------------

/// Issue #253 — the real OS MIDI backend (`ruviform-midi-device`'s
/// `SystemBackend`) driving the identical `InputTx::pump` →
/// `InputStage` → status-mirror path the `MockBackend` test covers
/// deterministically: enumerate → open → repeated pump (observing
/// `dropped`/`unsupported` counter transparency end-to-end) →
/// process → close → `StreamInvalidated`.
///
/// Manual opt-in: `cargo test -p ruviform-audio-plugin-bridge --
/// --ignored` (canonical mise lane registration is a recorded
/// follow-up — `mise.toml` is outside this slice's edit scope).
/// Honest exit on platforms without a backend
/// (`DeviceError::Unsupported`) or with no input ports; hardware
/// traffic is recorded, never asserted.
#[test]
#[ignore = "requires an OS MIDI input device; manual opt-in check"]
fn real_midi_device_drives_input_tx_to_stage() {
    let ports = match ruviform_midi_device::backend().input_ports() {
        Ok(ports) => ports,
        Err(err) => {
            eprintln!("OS MIDI backend unavailable ({err:?}) — nothing to verify");
            return;
        }
    };
    let Some(port) = ports.into_iter().next() else {
        eprintln!("no MIDI input ports — nothing to verify");
        return;
    };
    let info = port.info();
    eprintln!(
        "port: id={} name={:?} origin={:?} ump={} timestamps={} domain={:?}",
        info.id,
        info.name,
        info.origin,
        info.capabilities.supports_ump,
        info.capabilities.provides_timestamps,
        info.capabilities.clock_domain,
    );

    let mut rig = build_rig(128, true);
    // Anchor the backend's declared stamp domain at frame 0 — the
    // WinMM subscription stamps in `TimeBase` 1/1000 s
    // (`midi-device/src/queue.rs::Shared::new`). Mapped offsets are
    // timing-dependent on real hardware and are recorded, not
    // asserted.
    if let Some(domain) = info.capabilities.clock_domain {
        rig.control
            .set_clock_anchor(Some(ClockAnchor {
                domain,
                stamp_ticks: 0,
                stamp_base: TimeBase::new(1, 1_000).unwrap(),
                frame: EngineFrame::new(ClockEpochId::new(0), 0),
                calibration: TimestampProvenance::Carried,
            }))
            .unwrap();
    }

    let mut input = port.open().expect("open declared port");
    // Pump for a short window so real traffic has a chance to arrive.
    let (mut total_received, mut total_pushed) = (0usize, 0usize);
    let (mut total_dropped, mut total_unsupported) = (0u64, 0u64);
    for _ in 0..10 {
        let stats = rig.input_tx.as_mut().unwrap().pump(&mut *input).unwrap();
        total_received += stats.received;
        total_pushed += stats.pushed;
        total_dropped += stats.backend_dropped;
        total_unsupported += stats.backend_unsupported;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // Counter transparency end-to-end: every per-call observation
    // must land on the `InputTx` cumulative counters — nothing may
    // die at the port boundary.
    assert_eq!(
        rig.input_tx.as_ref().unwrap().dropped_backend(),
        total_dropped
    );
    assert_eq!(
        rig.input_tx.as_ref().unwrap().unsupported_backend(),
        total_unsupported
    );

    feed(&mut rig.rt, rig.src_out, 256);
    rig.rt.process(256, &RtContext::mint()).expect("process");
    let snap = rig.control.snapshot();
    eprintln!(
        "observed: received={total_received} pushed={total_pushed} \
         dropped={total_dropped} unsupported={total_unsupported} \
         stage_counters={:?}",
        snap.input
    );

    input.close();
    // `close` invalidates the subscription — a further pump reports
    // the backend error instead of delivering stale events.
    assert!(matches!(
        rig.input_tx.as_mut().unwrap().pump(&mut *input),
        Err(DeviceError::StreamInvalidated)
    ));
}
