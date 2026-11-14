//! Issue #562 RF-CB-01 — Generic Alternate Evaluation Context headless
//! `PoC`. Two candidate graph states are evaluated under one fixed input
//! using only the existing typed-runtime (`GraphRuntime`/`RtRuntime`),
//! offline-render (`PreparedRender`/`render`), and probe
//! (`pair`/`ProbeBroker`/`align_observation_points`) APIs — no new
//! generic API is introduced by this slice.
//!
//! Case IDs match `evidence/rf562/` and `docs/test-specs/issue-562-cb-poc.md`.
mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use assert_no_alloc::assert_no_alloc;
use common::*;

use ruviform_audio::render::{
    FeedContext, InputFeeder, OutputFacts, PreflightError, PreparedRender, RenderError,
    RenderOutput, RenderReceipt, RenderSpec, RenderTransport, StagedCommand, TailEvidence,
    TailSpec, render,
};
use ruviform_audio::typed::probe::{
    AlignPolicy, Alignment, AlignmentBasis, CoverageEvent, FrameInterval, GenerationRange,
    IncomparableReason, LatencyKnowledge, ObservationCoverage, ObservationFormat, ObservationSpan,
    ProbeBroker, ProbeClockMark, ProbeCollector, ProbeConfig, ProbeEvent, ProbeId, ProbeMode,
    ProbePoint, ProbePrecision, ProbeTimeContext, RecordMeta, StatusRecord,
    align_observation_points, pair,
};
use ruviform_audio::typed::{
    Compiler, GraphProcessor, GraphRuntime, NodeEvent, NodeIo, OfflineRenderSupport, RtRuntime,
    Slot,
};
use ruviform_audio_core::ChannelLayout;
use ruviform_exec::{CancellationToken, Progress};
use ruviform_graph::{Direction, GraphBuilder, GraphSpec, NodeId, NodeKind, PortId, PortRole};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, SealedFile, StagedFile};
use ruviform_media_core::{
    BlockContract, ClockAnchor, ClockEpochId, DeliveryPolicy, EngineFrame, InBlockTransportSupport,
    MediaDuration, MediaRange, MediaTime, ScheduleBudget, ScheduledEvent, TimeBase,
    TimestampProvenance, TransportCommand, TransportScheduler,
};
use ruviform_media_io::{Encoder, Error as MediaError, Frame, Muxer, Packet, StreamInfo};
use ruviform_rt::RtContext;

const FRAMES: usize = 64;
const BLOCKS: usize = 8;
const TOTAL: usize = FRAMES * BLOCKS;
const RATE: u32 = 48_000;
/// Shared engine epoch both candidate cursor series are anchored to —
/// the consumer-declared "same fixed input timeline" evidence.
const EVAL_EPOCH: u64 = 42;

fn ctx() -> RtContext {
    RtContext::mint()
}

/// Deterministic fixed input shared by every candidate: channel `c`,
/// global frame `p` → bounded repeating pattern in (-0.75, 0.75).
#[allow(clippy::cast_precision_loss)]
fn signal(channels: usize, len: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| {
            (0..len)
                .map(|p| (((p * 7 + c * 31) % 97) as f32 / 97.0) * 1.5 - 0.75)
                .collect()
        })
        .collect()
}

/// Non-repeating deterministic signal for window-membership asserts —
/// LCG high bits, period far beyond the fixture length (unlike
/// [`signal`]'s period-97 pattern).
#[allow(clippy::cast_precision_loss)]
fn signal_nr(channels: usize, len: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| {
            let mut s = 0x9E37_79B9_7F4A_7C15u64 ^ u64::try_from(c).unwrap();
            (0..len)
                .map(|_| {
                    s = s
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    ((s >> 33) % 1024) as f32 / 1024.0 - 0.5
                })
                .collect()
        })
        .collect()
}

/// Compiles `spec` once per call and publishes a fresh isolated
/// `GraphRuntime`+`RtRuntime` pair — the "candidate" construction.
/// Candidates share the validated graph shape and fixed input but own
/// every mutable processor/buffer.
fn build_runtime(
    spec: &GraphSpec,
    proc: NodeId,
    dsp: impl GraphProcessor + 'static,
) -> (GraphRuntime, RtRuntime) {
    let g = validate(spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(proc, Box::new(dsp));
    let cfg = config(FRAMES);
    let (mut grt, rt) = GraphRuntime::new(&cfg, 8, 8);
    let compiled = Compiler::new(cfg)
        .compile(
            &g,
            &mut factory,
            grt.registry(),
            grt.allocator(),
            grt.generation() + 1,
        )
        .expect("candidate compiles");
    grt.publish(compiled).expect("candidate publishes");
    (grt, rt)
}

/// One process block: feed `data` at `src_out`, process `FRAMES`,
/// return the drained sink planes.
fn block(
    rt: &mut RtRuntime,
    src_out: PortId,
    sink_in: PortId,
    data: &[Vec<f32>],
    offset: usize,
) -> Vec<Vec<f32>> {
    let l = &data[0][offset..offset + FRAMES];
    let r = data.get(1).map_or(&[][..], |p| &p[offset..offset + FRAMES]);
    feed(rt, src_out, &[l, r]);
    rt.process(FRAMES, &ctx()).expect("process");
    drain(rt, sink_in)
}

// ------------------------------------------------------------------
// CB-01: two candidates, one fixed input — state isolation and
// deterministic comparability.
// ------------------------------------------------------------------

#[test]
fn cb01_state_isolation_and_deterministic_compare() {
    let c = chain();
    let log_a = Arc::new(Mutex::new(Vec::new()));
    let log_b = Arc::new(Mutex::new(Vec::new()));
    let log_a2 = Arc::new(Mutex::new(Vec::new()));
    // Candidate A: gain 1.0. Candidate B: gain 0.5. Same graph spec,
    // same fixed input — only the candidate state differs.
    let (_ga, mut rt_a) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 1.0, log_a));
    let (_gb, mut rt_b) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 0.5, log_b));
    let input = signal(2, TOTAL);

    let mut out_a = Vec::new();
    let mut out_b = Vec::new();
    for k in 0..BLOCKS {
        let off = k * FRAMES;
        out_a.push(block(&mut rt_a, c.src_out, c.sink_in, &input, off));
        out_b.push(block(&mut rt_b, c.src_out, c.sink_in, &input, off));
    }

    // MockDsp: out = in·gain + state, state = per-instance process
    // count. Both candidates see identical call counts, so the state
    // term is equal and the gain term separates them exactly.
    for (k, (ba, bb)) in out_a.iter().zip(&out_b).enumerate() {
        let state = f32::from(u16::try_from(k + 1).unwrap());
        for ch in 0..2 {
            for f in 0..FRAMES {
                let x = input[ch][k * FRAMES + f];
                let va = ba[ch][f];
                let vb = bb[ch][f];
                assert!(
                    (va - (x + state)).abs() < f32::EPSILON,
                    "A ch{ch} blk{k} f{f}: {va} != {}",
                    x + state
                );
                assert!(
                    (vb - (x * 0.5 + state)).abs() < f32::EPSILON,
                    "B ch{ch} blk{k} f{f}: {vb}"
                );
            }
        }
    }

    // Isolation: a solo re-run of candidate A is bit-identical to the
    // interleaved run — B's processing never leaked into A's state.
    let (_ga2, mut rt_a2) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 1.0, log_a2));
    let mut solo = Vec::new();
    for k in 0..BLOCKS {
        solo.push(block(&mut rt_a2, c.src_out, c.sink_in, &input, k * FRAMES));
    }
    assert_eq!(solo, out_a, "candidate A output changed by B's activity");
}

// ------------------------------------------------------------------
// CB-02: preview A/B switching — main path and its observation
// stream are untouched; RT path allocates nothing.
// ------------------------------------------------------------------

#[test]
fn cb02_preview_switch_no_main_or_recording_side_effects() {
    let c = chain();
    let log_m = Arc::new(Mutex::new(Vec::new()));
    let log_p1 = Arc::new(Mutex::new(Vec::new()));
    let log_p2 = Arc::new(Mutex::new(Vec::new()));
    let (mut grt_m, mut rt_m) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 1.0, log_m));
    let (_grt_p1, mut rt_p1) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 0.5, log_p1));
    let (mut grt_p2, mut rt_p2) = build_runtime(&c.spec, c.proc, MockDsp::new(0, 0.25, log_p2));
    // 10 blocks: 2 main-only + 3 P1 + 3 P2 + 2 main-only resume,
    // plus one flush block after the event section.
    let input = signal(2, 11 * FRAMES);

    // "Recording" = the main path's drained sink output; preview
    // evaluation must not alter it.
    let mut main_rec = Vec::new();
    // Phase 1: main alone for 2 blocks (control prefix).
    for k in 0..2 {
        main_rec.push(block(&mut rt_m, c.src_out, c.sink_in, &input, k * FRAMES));
    }
    // Phase 2: preview candidate P1 auditioned while main keeps
    // running — interleaved blocks, main output still recorded.
    let mut p1_outs = Vec::new();
    for k in 2..2 + 3 {
        main_rec.push(block(&mut rt_m, c.src_out, c.sink_in, &input, k * FRAMES));
        p1_outs.push(block(
            &mut rt_p1,
            c.src_out,
            c.sink_in,
            &input,
            (k - 2) * FRAMES,
        ));
    }
    // Phase 3: A/B switch — P2 auditioned, P1 untouched this phase.
    let mut p2_outs = Vec::new();
    for k in 5..5 + 3 {
        main_rec.push(block(&mut rt_m, c.src_out, c.sink_in, &input, k * FRAMES));
        p2_outs.push(block(
            &mut rt_p2,
            c.src_out,
            c.sink_in,
            &input,
            (k - 5) * FRAMES,
        ));
    }
    // Phase 4: switch back — main resumes exactly where it left off.
    for k in 8..10 {
        main_rec.push(block(&mut rt_m, c.src_out, c.sink_in, &input, k * FRAMES));
    }

    // Control: main driven alone through the same schedule.
    let (_gc, mut rt_c) = build_runtime(
        &c.spec,
        c.proc,
        MockDsp::new(0, 1.0, Arc::new(Mutex::new(Vec::new()))),
    );
    let mut control = Vec::new();
    for k in 0..10 {
        control.push(block(&mut rt_c, c.src_out, c.sink_in, &input, k * FRAMES));
    }
    assert_eq!(
        main_rec, control,
        "preview A/B evaluation disturbed the main path or its recording"
    );

    // Preview outputs are candidate-correct: P1 saw 3 blocks of its
    // own fixed input at gain 0.5, P2 saw 3 at gain 0.25 — a fresh
    // candidate reproduces them bit-identically.
    let replay = |gain: f32| {
        let (_g, mut rt) = build_runtime(
            &c.spec,
            c.proc,
            MockDsp::new(0, gain, Arc::new(Mutex::new(Vec::new()))),
        );
        (0..3)
            .map(|k| block(&mut rt, c.src_out, c.sink_in, &input, k * FRAMES))
            .collect::<Vec<_>>()
    };
    assert_eq!(p1_outs, replay(0.5), "P1 preview output not reproducible");
    assert_eq!(p2_outs, replay(0.25), "P2 preview output not reproducible");

    // Candidate-scoped events: a generation-matching event reaches
    // only its own runtime; a stale-generation event is counted, not
    // delivered.
    let fresh_p2 = NodeEvent {
        generation: grt_p2.generation(),
        node: c.proc,
        opcode: 0,
        value: 2.0,
    };
    grt_p2.send_event(fresh_p2).expect("send to P2");
    let stale_m = NodeEvent {
        generation: 0,
        node: c.proc,
        opcode: 0,
        value: 9.0,
    };
    grt_m.send_event(stale_m).expect("send stale to main");
    let p2_after = block(&mut rt_p2, c.src_out, c.sink_in, &input, 3 * FRAMES);
    // gain now 2.0, state = 4 (three prior blocks + this one).
    let x = input[0][3 * FRAMES];
    assert!(
        (p2_after[0][0] - (x * 2.0 + 4.0)).abs() < f32::EPSILON,
        "P2 event applied: {}",
        p2_after[0][0]
    );
    // One more main block flushes the queued stale event — counted,
    // never delivered to the DSP.
    block(&mut rt_m, c.src_out, c.sink_in, &input, 10 * FRAMES);
    assert_eq!(
        grt_m.status().snapshot().stale_events,
        1,
        "stale event on main counted and dropped"
    );

    // RT NoAlloc: audited process calls on both a main-runtime and a
    // candidate-runtime allocate nothing after warm-up.
    let data: Vec<Vec<f32>> = signal(2, FRAMES);
    let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
    feed(&mut rt_m, c.src_out, &refs);
    rt_m.process(FRAMES, &ctx()).expect("main warm-up");
    feed(&mut rt_p1, c.src_out, &refs);
    rt_p1.process(FRAMES, &ctx()).expect("candidate warm-up");
    feed(&mut rt_m, c.src_out, &refs);
    assert_no_alloc(|| rt_m.process(FRAMES, &ctx()).expect("main audited"));
    feed(&mut rt_p1, c.src_out, &refs);
    assert_no_alloc(|| rt_p1.process(FRAMES, &ctx()).expect("candidate audited"));
}

// ------------------------------------------------------------------
// CB-03: multi-bus graph — Sidechain-role input flows independently;
// generation-tagged events are candidate-scoped.
// ------------------------------------------------------------------

/// key·(1 + side·sidechain) processor reading input slots 0 (main)
/// and 1 (sidechain role).
struct KeyDsp {
    side: f32,
}

impl GraphProcessor for KeyDsp {
    fn event(&mut self, ev: &NodeEvent, _ctx: &RtContext) {
        self.side = ev.value;
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let main = io.input(0);
        let key = io.input(1);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for ch in 0..out.channels() {
                let Some(plane) = out.channel_mut(ch) else {
                    continue;
                };
                let m = main.as_ref().and_then(|v| v.channel(ch));
                let k = key.as_ref().and_then(|v| v.channel(ch));
                for (i, d) in plane.iter_mut().enumerate() {
                    let mv = m.map_or(0.0, |s| s[i]);
                    let kv = k.map_or(0.0, |s| s[i]);
                    *d = mv * (1.0 + self.side * kv);
                }
            }
        }
    }
}

struct SidechainChain {
    spec: GraphSpec,
    proc: NodeId,
    main_out: PortId,
    key_out: PortId,
    sink_in: PortId,
}

/// `main_src` ──► proc.in[Main]      `side_src` ──► proc.in[Sidechain]
///                        proc.out[Main] ──► sink
fn sidechain_chain() -> SidechainChain {
    let mut b = GraphBuilder::new();
    let (src_m, sm) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Main, stereo())],
    ));
    let (src_k, sk) = b.add_node(&node(
        NodeKind::Source,
        vec![port(Direction::Output, PortRole::Sidechain, stereo())],
    ));
    let (proc, pp) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Input, PortRole::Sidechain, stereo()),
            port(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (sink, kp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    b.connect(&conn(pref(src_m, sm[0]), pref(proc, pp[0])));
    b.connect(&conn(pref(src_k, sk[0]), pref(proc, pp[1])));
    b.connect(&conn(pref(proc, pp[2]), pref(sink, kp[0])));
    SidechainChain {
        spec: b.build(),
        proc,
        main_out: sm[0],
        key_out: sk[0],
        sink_in: kp[0],
    }
}

#[test]
fn cb03_sidechain_bus_and_generation_scoped_events() {
    let c = sidechain_chain();
    let (mut grt_a, mut rt_a) = build_runtime(&c.spec, c.proc, KeyDsp { side: 0.5 });
    let (mut grt_b, mut rt_b) = build_runtime(&c.spec, c.proc, KeyDsp { side: 2.0 });
    let main_sig = signal(2, FRAMES);
    let key_sig = signal(2, FRAMES);
    let mrefs: Vec<&[f32]> = main_sig.iter().map(Vec::as_slice).collect();
    let krefs: Vec<&[f32]> = key_sig.iter().map(Vec::as_slice).collect();

    let drive = |rt: &mut RtRuntime| {
        feed(rt, c.main_out, &mrefs);
        feed(rt, c.key_out, &krefs);
        rt.process(FRAMES, &ctx()).expect("process");
        drain(rt, c.sink_in)
    };

    let out_a = drive(&mut rt_a);
    let out_b = drive(&mut rt_b);
    for ch in 0..2 {
        for f in 0..FRAMES {
            let expected_a = main_sig[ch][f] * (1.0 + 0.5 * key_sig[ch][f]);
            let expected_b = main_sig[ch][f] * (1.0 + 2.0 * key_sig[ch][f]);
            assert!(
                (out_a[ch][f] - expected_a).abs() < 1e-5,
                "A sidechain mix ch{ch} f{f}"
            );
            assert!(
                (out_b[ch][f] - expected_b).abs() < 1e-5,
                "B sidechain mix ch{ch} f{f}"
            );
        }
    }

    // Sidechain isolation: silencing the key bus changes output only
    // through the declared mix — candidate A keeps its own coefficient.
    let zeros = [vec![0.0f32; FRAMES], vec![0.0f32; FRAMES]];
    let zrefs: Vec<&[f32]> = zeros.iter().map(Vec::as_slice).collect();
    feed(&mut rt_a, c.main_out, &mrefs);
    feed(&mut rt_a, c.key_out, &zrefs);
    rt_a.process(FRAMES, &ctx()).expect("process");
    let out_a_z = drain(&mut rt_a, c.sink_in);
    for ch in 0..2 {
        for f in 0..FRAMES {
            assert!(
                (out_a_z[ch][f] - main_sig[ch][f]).abs() < 1e-5,
                "zero sidechain → unity mix ch{ch} f{f}"
            );
        }
    }

    // Generation-scoped events: B's fresh event applies to B only.
    grt_b
        .send_event(NodeEvent {
            generation: grt_b.generation(),
            node: c.proc,
            opcode: 0,
            value: 0.0,
        })
        .expect("send to B");
    let stale_a = NodeEvent {
        generation: 0,
        node: c.proc,
        opcode: 0,
        value: 9.0,
    };
    grt_a.send_event(stale_a).expect("send stale to A");
    // One more block flushes A's queued stale event.
    let _ = drive(&mut rt_a);

    feed(&mut rt_b, c.main_out, &mrefs);
    feed(&mut rt_b, c.key_out, &krefs);
    rt_b.process(FRAMES, &ctx()).expect("process B");
    let out_b_ev = drain(&mut rt_b, c.sink_in);
    // side = 0 → out = main exactly.
    assert_eq!(out_b_ev[0][..FRAMES], main_sig[0][..]);
    assert_eq!(grt_a.status().snapshot().stale_events, 1);
}

// ------------------------------------------------------------------
// CB-04: offline render — two PreparedRender candidates over one
// fixed feeder/transport; PDC/latency, tail, failure isolation,
// seek/loop determinism.
// ------------------------------------------------------------------

/// Stateless gain with declared latency and an offline-support gate —
/// candidate identity is (gain, latency, support).
struct GainDsp {
    gain: f32,
    latency: u32,
    support: OfflineRenderSupport,
}

impl GainDsp {
    fn verified(gain: f32, latency: u32) -> Self {
        Self {
            gain,
            latency,
            support: OfflineRenderSupport::Verified,
        }
    }
}

impl GraphProcessor for GainDsp {
    fn latency(&self) -> u32 {
        self.latency
    }

    fn offline_support(&self) -> OfflineRenderSupport {
        self.support
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let input = io.input(0);
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for ch in 0..out.channels() {
                let Some(plane) = out.channel_mut(ch) else {
                    continue;
                };
                let src = input.as_ref().and_then(|v| v.channel(ch));
                for (i, d) in plane.iter_mut().enumerate() {
                    *d = src.map_or(0.0, |s| s[i]) * self.gain;
                }
            }
        }
    }
}

/// Feeds `signal[c][position + j]` into the boundary input slot.
struct SignalFeeder {
    data: Vec<Vec<f32>>,
}

impl InputFeeder for SignalFeeder {
    fn feed(&mut self, _port: PortId, ctx: &FeedContext, slot: &mut Slot) {
        let pos = usize::try_from(ctx.position_ticks.max(0)).unwrap_or(0);
        for ch in 0..usize::from(slot.channels) {
            let Some(plane) = slot.data.channel_mut(ch) else {
                continue;
            };
            let src = &self.data[ch.min(self.data.len() - 1)];
            for (j, d) in plane[..ctx.frames].iter_mut().enumerate() {
                *d = src.get(pos + j).copied().unwrap_or(0.0);
            }
        }
    }
}

/// Captured emission evidence: appended channel planes and the pts of
/// each emitted run.
#[derive(Default)]
struct Captured {
    planes: Vec<Vec<f32>>,
    runs: Vec<(i64, usize)>,
}

/// Encoder that copies `AudioFrame` planes into `out` and yields no
/// packets — the semantic-path sink.
struct CaptureEncoder {
    out: Arc<Mutex<Captured>>,
}

impl Encoder for CaptureEncoder {
    fn send_frame(&mut self, frame: Frame) -> ruviform_media_io::Result<()> {
        if let Frame::Audio(audio) = frame {
            let mut out = self.out.lock().unwrap();
            let buffer = audio.buffer();
            if out.planes.is_empty() {
                out.planes = vec![Vec::new(); buffer.channels()];
            }
            out.runs.push((audio.pts().time.value(), buffer.frames()));
            for (c, plane) in out.planes.iter_mut().enumerate() {
                plane.extend_from_slice(buffer.channel(c).unwrap());
            }
        }
        Ok(())
    }

    fn receive_packet(&mut self) -> ruviform_media_io::Result<Option<Packet>> {
        Ok(None)
    }
}

/// Muxer that accepts every packet and finishes cleanly.
struct NullMuxer {
    streams: Vec<StreamInfo>,
}

impl Muxer for NullMuxer {
    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    fn write_packet(&mut self, _packet: Packet) -> ruviform_media_io::Result<()> {
        Ok(())
    }

    fn finish(&mut self) -> ruviform_media_io::Result<()> {
        Ok(())
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the driver bound requires a fallible constructor"
)]
fn null_muxer(_staged: &mut StagedFile) -> Result<Box<dyn Muxer + '_>, MediaError> {
    Ok(Box::new(NullMuxer {
        streams: Vec::new(),
    }))
}

/// Unique scratch dir under the OS temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> (Self, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "rvf-cb562-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("out.wav");
        (Self(dir), dest)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const NO_OVERWRITE: PublishOptions =
    PublishOptions::new(PublishMode::CreateNew, Durability::FileAndDirectorySync);

fn rate() -> ruviform_audio_core::SampleRate {
    ruviform_audio_core::SampleRate::new(RATE).unwrap()
}

fn base() -> TimeBase {
    rate().time_base()
}

fn mt(ticks: i64) -> MediaTime {
    MediaTime::new(ticks, base())
}

fn prepared(c: &Chain, dsp: impl GraphProcessor + 'static) -> PreparedRender {
    let g = validate(&c.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(c.proc, Box::new(dsp));
    PreparedRender::compile(&g, &mut factory, config(768), 8, 64).expect("compile")
}

fn transport<P>(max_frames: u32) -> RenderTransport<P> {
    RenderTransport {
        scheduler: TransportScheduler::new(
            base(),
            BlockContract::new(1, max_frames, InBlockTransportSupport::Unknown),
            DeliveryPolicy::Exact,
            ScheduleBudget::default(),
        ),
        commands: Vec::new(),
        plans: Vec::new(),
    }
}

fn play_at<P>(t: &mut RenderTransport<P>, frame: u64) {
    let epoch = t.scheduler.epoch();
    t.commands.push(StagedCommand {
        command: TransportCommand::Play,
        due: EngineFrame::new(epoch, frame),
    });
}

fn stage<P>(t: &mut RenderTransport<P>, frame: u64, command: TransportCommand) {
    let epoch = t.scheduler.epoch();
    t.commands.push(StagedCommand {
        command,
        due: EngineFrame::new(epoch, frame),
    });
}

fn spec(c: &Chain, range: (u64, u64), tail: TailSpec, job: (u64, u64), chunk: u32) -> RenderSpec {
    RenderSpec {
        sample_rate: rate(),
        layout: ChannelLayout::Stereo,
        output_port: c.sink_in,
        range_start: range.0,
        range_end: range.1,
        tail,
        job_start: job.0,
        job_end: job.1,
        callback_chunk: chunk,
    }
}

type VerifyFn = fn(&mut SealedFile, &OutputFacts) -> Result<(), MediaError>;
type MuxerCtor = for<'a> fn(&'a mut StagedFile) -> Result<Box<dyn Muxer + 'a>, MediaError>;
type PublishFn = fn(
    SealedFile,
    PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, ruviform_io_file::PublishFailure>;

fn capture_output(
    dest: &Path,
    captured: Arc<Mutex<Captured>>,
    options: PublishOptions,
) -> RenderOutput<MuxerCtor, VerifyFn, PublishFn> {
    RenderOutput {
        staged: StagedFile::create(dest).expect("staged file"),
        make_muxer: null_muxer,
        encoder: Box::new(CaptureEncoder { out: captured }),
        publish_options: options,
        verify: None,
        publish: SealedFile::publish,
    }
}

fn no_events(_: &ScheduledEvent<u32>, _: u64) -> Option<NodeEvent> {
    None
}

struct Rendered {
    receipt: RenderReceipt,
    captured: Captured,
}

fn run_render(
    p: &mut PreparedRender,
    t: &mut RenderTransport<u32>,
    sp: &RenderSpec,
    data: &[Vec<f32>],
    tag: &str,
) -> Rendered {
    let (scratch, dest) = Scratch::new(tag);
    let _keep = scratch;
    let mut feeder = SignalFeeder {
        data: data.to_vec(),
    };
    let captured = Arc::new(Mutex::new(Captured::default()));
    let receipt = render(
        p,
        t,
        sp,
        &mut feeder,
        capture_output(&dest, Arc::clone(&captured), NO_OVERWRITE),
        &Progress::new(),
        &CancellationToken::new(),
        &mut no_events,
    )
    .expect("render succeeds");
    let captured = Arc::into_inner(captured).unwrap().into_inner().unwrap();
    Rendered { receipt, captured }
}

/// CB-04a: two candidates (gain 1.0/latency 0 vs gain 0.5/latency 32)
/// over one fixed feeder/transport — deterministic, PDC-aligned,
/// tail-explicit, independently receipted.
#[test]
fn cb04a_offline_render_two_candidates_pdc_tail() {
    let c = chain();
    let len = 512usize;
    let data = signal(2, len + 128);
    let range = (0u64, 512u64);
    let tail_frames = 32u64;

    // Candidate A: gain 1.0, latency 0. Candidate B: gain 0.5,
    // declared latency 32 — PDC shifts its emitted window content.
    let mut pa = prepared(&c, GainDsp::verified(1.0, 0));
    let mut pb = prepared(&c, GainDsp::verified(0.5, 32));

    let mut ta = transport::<u32>(128);
    play_at(&mut ta, 0);
    let sa = spec(&c, range, TailSpec::Known(tail_frames), (0, 512 + 32), 0);
    let ra = run_render(&mut pa, &mut ta, &sa, &data, "cb04a-a");

    let mut tb = transport::<u32>(128);
    play_at(&mut tb, 0);
    // B's job must cover its declared latency warm-up + range + tail.
    let sb = spec(
        &c,
        range,
        TailSpec::Known(tail_frames),
        (0, 512 + 32 + 32),
        0,
    );
    let rb = run_render(&mut pb, &mut tb, &sb, &data, "cb04a-b");

    // Same emitted window, same tail evidence, different declared
    // total latency — the comparison metadata consumers need.
    assert_eq!(ra.receipt.frames_emitted, 512 + 32);
    assert_eq!(rb.receipt.frames_emitted, 512 + 32);
    assert_eq!(ra.receipt.tail_evidence, TailEvidence::Known);
    assert_eq!(rb.receipt.tail_evidence, TailEvidence::Known);
    assert_eq!(ra.receipt.range_end_with_tail, 544);
    assert_eq!(rb.receipt.range_end_with_tail, 544);
    assert_eq!(pa.total_latency(), 0);
    assert_eq!(pb.total_latency(), 32);

    // A emitted[p] = signal[p]; B emitted[p] = signal[p + 32]·0.5 —
    // latency is reported, never hidden.
    for (ch, (plane_a, plane_b)) in ra
        .captured
        .planes
        .iter()
        .zip(rb.captured.planes.iter())
        .enumerate()
    {
        for (i, (&va, &vb)) in plane_a.iter().zip(plane_b.iter()).enumerate() {
            let expected_a = data[ch][i];
            let expected_b = data[ch][i + 32] * 0.5;
            assert!(
                (va - expected_a).abs() < 1e-6,
                "A ch{ch} i{i}: {va} != {expected_a}"
            );
            assert!(
                (vb - expected_b).abs() < 1e-6,
                "B ch{ch} i{i}: {vb} != {expected_b}"
            );
        }
    }

    // Determinism: re-rendering candidate A reproduces bit-identical
    // planes and run timestamps.
    let mut pa2 = prepared(&c, GainDsp::verified(1.0, 0));
    let mut ta2 = transport::<u32>(128);
    play_at(&mut ta2, 0);
    let ra2 = run_render(&mut pa2, &mut ta2, &sa, &data, "cb04a-a2");
    assert_eq!(ra2.captured.planes, ra.captured.planes);
    assert_eq!(ra2.captured.runs, ra.captured.runs);
}

/// CB-04b: a candidate that fails the offline-support preflight is
/// rejected without disturbing the other candidate.
#[test]
fn cb04b_preflight_failure_isolated() {
    let c = chain();
    let data = signal_nr(2, 1024);

    // Failure isolation: candidate U declares Unknown offline support
    // → bounded Preflight rejection; candidate G still renders.
    let mut pu = prepared(
        &c,
        GainDsp {
            gain: 1.0,
            latency: 0,
            support: OfflineRenderSupport::Unknown,
        },
    );
    let mut tu = transport::<u32>(128);
    play_at(&mut tu, 0);
    let su = spec(&c, (0, 256), TailSpec::None, (0, 256), 0);
    let (scratch_u, dest_u) = Scratch::new("cb04b-u");
    let _keep_u = scratch_u;
    let mut feeder_u = SignalFeeder { data: data.clone() };
    let captured_u = Arc::new(Mutex::new(Captured::default()));
    let err = render(
        &mut pu,
        &mut tu,
        &su,
        &mut feeder_u,
        capture_output(&dest_u, Arc::clone(&captured_u), NO_OVERWRITE),
        &Progress::new(),
        &CancellationToken::new(),
        &mut no_events,
    )
    .expect_err("Unknown support must fail preflight");
    assert!(
        matches!(
            err,
            RenderError::Preflight(PreflightError::OfflineUnsupported { .. })
        ),
        "expected OfflineUnsupported preflight, got {err:?}"
    );
    assert!(
        captured_u.lock().unwrap().planes.is_empty(),
        "failed candidate emitted nothing"
    );

    // The good candidate is unaffected by the other's failure.
    let mut pg = prepared(&c, GainDsp::verified(1.0, 0));
    let mut tg = transport::<u32>(128);
    play_at(&mut tg, 0);
    let sg = spec(&c, (0, 256), TailSpec::None, (0, 256), 0);
    let rg = run_render(&mut pg, &mut tg, &sg, &data, "cb04b-g");
    assert_eq!(rg.receipt.frames_emitted, 256);
    for (got, want) in rg.captured.planes.iter().zip(data.iter()) {
        assert_eq!(&got[..256], &want[..256]);
    }
}

/// CB-04c: seek and loop transports are deterministic under
/// repetition and observably displace the emitted signal.
#[test]
fn cb04c_seek_loop_determinism() {
    let c = chain();
    let data = signal_nr(2, 1024);

    // Seek determinism: Play@0, Seek→position 512 due at frame 128.
    // Emission is pts-windowed on media position, so the range must
    // reach past the seek target for post-seek frames to emit.
    // Two identical transports must produce identical output; the
    // emitted planes must differ from the uninterrupted render and
    // contain the post-seek signal window while skipping the gap.
    let seek_spec = spec(&c, (0, 768), TailSpec::None, (0, 896), 0);
    let seek_render = |tag: &str, seek: bool| {
        let mut p = prepared(&c, GainDsp::verified(1.0, 0));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        if seek {
            stage(&mut t, 128, TransportCommand::Seek(mt(512)));
        }
        run_render(&mut p, &mut t, &seek_spec, &data, tag)
    };
    let rs0 = seek_render("cb04c-noseek", false);
    let rs1 = seek_render("cb04c-seek1", true);
    let rs2 = seek_render("cb04c-seek2", true);
    assert_eq!(
        rs1.captured.planes, rs2.captured.planes,
        "seek not deterministic"
    );
    assert_eq!(rs1.captured.runs, rs2.captured.runs);
    assert_ne!(
        rs1.captured.planes, rs0.captured.planes,
        "seek had no effect on output"
    );
    // The post-seek signal window appears contiguously in the output.
    let seek_win: &[f32] = &data[0][512..576];
    assert!(
        rs1.captured.planes[0].windows(64).any(|w| w == seek_win),
        "post-seek signal window not observed"
    );
    // The skipped region between the pre-seek and post-seek windows
    // never appears — the seek really displaced the position.
    let gap_win: &[f32] = &data[0][256..320];
    assert!(
        !rs1.captured.planes[0].windows(64).any(|w| w == gap_win),
        "skipped region leaked into output"
    );

    // Loop determinism: SetLoop[64,128) wraps the position; two runs
    // identical, the looped segment repeats, and the emitted pts show
    // the wrap (position reset).
    let loop_render = |tag: &str| {
        let mut p = prepared(&c, GainDsp::verified(1.0, 0));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        stage(
            &mut t,
            0,
            TransportCommand::SetLoop(MediaRange::new(mt(64), MediaDuration::new(64, base()))),
        );
        let sp = spec(&c, (0, 256), TailSpec::None, (0, 384), 0);
        run_render(&mut p, &mut t, &sp, &data, tag)
    };
    let rl1 = loop_render("cb04c-loop1");
    let rl2 = loop_render("cb04c-loop2");
    assert_eq!(
        rl1.captured.planes, rl2.captured.planes,
        "loop not deterministic"
    );
    assert_eq!(rl1.captured.runs, rl2.captured.runs);
    // The 64-frame loop body appears at least twice contiguously —
    // impossible in the linear render of the same spec.
    let linear_render = |tag: &str| {
        let mut p = prepared(&c, GainDsp::verified(1.0, 0));
        let mut t = transport::<u32>(128);
        play_at(&mut t, 0);
        let sp = spec(&c, (0, 256), TailSpec::None, (0, 384), 0);
        run_render(&mut p, &mut t, &sp, &data, tag)
    };
    let rl0 = linear_render("cb04c-linear");
    let loop_body: &[f32] = &data[0][64..128];
    assert!(
        rl1.captured.planes[0]
            .windows(64)
            .filter(|w| *w == loop_body)
            .count()
            >= 2,
        "looped segment did not repeat"
    );
    assert_eq!(
        rl0.captured.planes[0]
            .windows(64)
            .filter(|w| *w == loop_body)
            .count(),
        1,
        "linear render unexpectedly repeats the loop body"
    );
}

// ------------------------------------------------------------------
// CB-05: probe observations from two candidate runtimes — coverage
// reconstruction and the three-valued alignment contract across
// independent clock domains.
// ------------------------------------------------------------------

struct ProbeChain {
    spec: GraphSpec,
    proc: NodeId,
    src_out: PortId,
    proc_out: PortId,
}

/// Source → processor → sink with `ProbePoint` on the processor's
/// output (same shape as the Issue #185 fixture).
fn probe_chain() -> ProbeChain {
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
    b.connect(&conn(pref(src, sp[0]), pref(proc, pp[0])));
    b.connect(&conn(pref(proc, pp[1]), pref(sink, kp[0])));
    ProbeChain {
        spec: b.build(),
        proc,
        src_out: sp[0],
        proc_out: pp[1],
    }
}

struct ProbeRig {
    grt: GraphRuntime,
    rt: RtRuntime,
    broker: ProbeBroker,
    collector: ProbeCollector,
    probe: ProbeId,
    point: ProbePoint,
}

fn probe_rig(c: &ProbeChain, dsp: impl GraphProcessor + 'static) -> ProbeRig {
    let (grt, mut rt) = build_runtime(&c.spec, c.proc, dsp);
    let parts = pair(&ProbeConfig::default());
    rt.set_probe_deck(parts.deck);
    let mut broker = parts.broker;
    let collector = parts.collector;
    let point = ProbePoint {
        node: c.proc,
        port: c.proc_out,
    };
    let probe = broker
        .install(&grt, &c.spec, point, ProbeMode::Monitor)
        .expect("probe installs");
    broker.wire_clock_mark().expect("clock mark wired");
    ProbeRig {
        grt,
        rt,
        broker,
        collector,
        probe,
        point,
    }
}

/// Owned drained evidence for one probe series.
struct Series {
    probe_id: ProbeId,
    point: ProbePoint,
    coverage: ObservationCoverage,
    interval: FrameInterval,
    generation_range: GenerationRange,
    format: ObservationFormat,
    latency: LatencyKnowledge,
    clock_mark: ProbeClockMark,
    context: ProbeTimeContext,
    data_records: u64,
}

impl Series {
    fn span(&self) -> ObservationSpan<'_> {
        ObservationSpan {
            probe: self.probe_id,
            point: self.point,
            clock_mark: self.clock_mark,
            interval: self.interval,
            generation_range: self.generation_range,
            format: self.format,
            latency: self.latency,
            coverage: &self.coverage,
            context: Some(&self.context),
            precision: ProbePrecision::F32Rt,
        }
    }
}

// probe_id/point are needed by span(); store them alongside.
// (Rust has no field init shorthand across structs — extend Series.)

fn run_probe_series(rig: &mut ProbeRig, c: &ProbeChain, data: &[Vec<f32>]) {
    for k in 0..BLOCKS {
        let off = k * FRAMES;
        let l = &data[0][off..off + FRAMES];
        let r = &data[1][off..off + FRAMES];
        feed(&mut rig.rt, c.src_out, &[l, r]);
        rig.rt.process(FRAMES, &ctx()).expect("process");
    }
}

/// Drains the collector and reconstructs the span facts — mirrors
/// `build_view` in `typed_probe_time_align.rs`, kept local because test
/// crates cannot share items.
fn collect_series(rig: &mut ProbeRig) -> (Series, u64) {
    let lost_before = rig.collector.counts().lost_records;
    let mut metas: Vec<RecordMeta> = Vec::new();
    let mut statuses: Vec<StatusRecord> = Vec::new();
    rig.collector.drain(|ev| match ev {
        ProbeEvent::Metric(m) => metas.push(m.meta),
        ProbeEvent::Status(s) => statuses.push(s),
        _ => {}
    });
    let lost_after = rig.collector.counts().lost_records;
    let lost_delta = lost_after.saturating_sub(lost_before);

    let coverage = ObservationCoverage::reconstruct(
        rig.probe,
        lost_delta,
        metas
            .iter()
            .map(|&m| CoverageEvent::Data(m))
            .chain(statuses.iter().map(|&s| CoverageEvent::Status(s))),
    );

    let mut min_pos = u64::MAX;
    let mut max_end = 0_u64;
    let mut gen_first = u64::MAX;
    let mut gen_last = 0_u64;
    let mut first_meta: Option<RecordMeta> = None;
    for &m in &metas {
        if first_meta.is_none() {
            first_meta = Some(m);
        }
        min_pos = min_pos.min(m.sample_position);
        max_end = max_end.max(m.sample_position.saturating_add(u64::from(m.frames)));
        gen_first = gen_first.min(m.generation);
        gen_last = gen_last.max(m.generation);
    }
    for s in &statuses {
        gen_first = gen_first.min(s.generation);
        gen_last = gen_last.max(s.generation);
    }
    let (format, latency, clock_mark) = match first_meta {
        Some(m) => (
            ObservationFormat {
                channels: m.channels,
                sample_rate: m.sample_rate,
                channel_layout: ChannelLayout::Stereo,
            },
            LatencyKnowledge::Known {
                frames: u64::from(m.effective_latency),
            },
            m.clock,
        ),
        None => (
            ObservationFormat {
                channels: 0,
                sample_rate: 0,
                channel_layout: ChannelLayout::Stereo,
            },
            LatencyKnowledge::Unknown,
            ProbeClockMark::UNKNOWN,
        ),
    };
    (
        Series {
            coverage,
            interval: if min_pos == u64::MAX {
                FrameInterval::new(0, 0)
            } else {
                FrameInterval::new(min_pos, max_end)
            },
            generation_range: if gen_first == u64::MAX {
                GenerationRange::single(0)
            } else {
                GenerationRange {
                    first: gen_first,
                    last: gen_last,
                }
            },
            format,
            latency,
            clock_mark,
            context: rig.broker.time_context().clone(),
            data_records: metas.len() as u64,
            probe_id: rig.probe,
            point: rig.point,
        },
        lost_delta,
    )
}

#[test]
fn cb05_probe_cross_candidate_alignment() {
    let c = probe_chain();
    let data = signal(2, TOTAL);

    // Candidate A: gain 1.0, latency 0. Candidate B: gain 0.5,
    // declared latency 32 — each on its own runtime + probe pair.
    let mut ra = probe_rig(&c, MockDsp::new(0, 1.0, Arc::new(Mutex::new(Vec::new()))));
    let mut rb = probe_rig(&c, MockDsp::new(32, 0.5, Arc::new(Mutex::new(Vec::new()))));

    // Declare the engine frame rate backing each cursor series.
    ra.broker.time_context_mut().sample_rate = RATE;
    rb.broker.time_context_mut().sample_rate = RATE;

    run_probe_series(&mut ra, &c, &data);
    run_probe_series(&mut rb, &c, &data);

    let (sa, lost_a) = collect_series(&mut ra);
    let (sb, lost_b) = collect_series(&mut rb);
    assert_eq!(lost_a, 0);
    assert_eq!(lost_b, 0);
    assert!(sa.data_records > 0 && sb.data_records > 0);
    assert_eq!(sa.interval, FrameInterval::new(0, TOTAL as u64));
    assert_eq!(sb.interval, FrameInterval::new(0, TOTAL as u64));

    // Finding for the gap ledger: both candidates report plan
    // generation 1 (per-runtime counter) — generation alone cannot
    // distinguish candidates; the clock domain / runtime_instance can.
    assert_eq!(sa.generation_range, GenerationRange::single(1));
    assert_eq!(sb.generation_range, GenerationRange::single(1));
    assert_ne!(
        sa.clock_mark.domain, sb.clock_mark.domain,
        "each probe pair mints a unique cursor domain"
    );
    assert_ne!(
        sa.context.runtime_instance, sb.context.runtime_instance,
        "each pair has a distinct runtime instance identity"
    );
    // Both candidates sit at their runtime's first published
    // generation — the per-runtime counter is identical by design.
    assert_eq!(ra.grt.generation(), 1);
    assert_eq!(rb.grt.generation(), 1);

    let policy = AlignPolicy::default();

    // Without consumer-declared shared time base: two distinct cursor
    // domains cannot align — bounded Incomparable, never a guess.
    let al_none = align_observation_points(&sa.span(), &sb.span(), &policy);
    assert!(
        matches!(
            al_none,
            Alignment::Incomparable {
                reason: IncomparableReason::DomainMismatch | IncomparableReason::AnchorMissing
            }
        ),
        "unanchored cross-candidate spans must fail closed: {al_none:?}"
    );

    // Wire both series to one shared engine epoch — the consumer's
    // declaration that both candidates processed the same fixed input
    // timeline (the EvaluationInputIdentity analogue).
    for rig in [&mut ra, &mut rb] {
        let domain = rig.broker.time_context().domain;
        rig.broker.time_context_mut().anchors.push(ClockAnchor {
            domain,
            stamp_ticks: 0,
            stamp_base: base(),
            frame: EngineFrame::new(ClockEpochId::new(EVAL_EPOCH), 0),
            calibration: TimestampProvenance::Carried,
        });
    }
    // Refresh the cloned contexts inside the series.
    let mut sa = sa;
    let mut sb = sb;
    sa.context = ra.broker.time_context().clone();
    sb.context = rb.broker.time_context().clone();

    let al = align_observation_points(&sa.span(), &sb.span(), &policy);
    match &al {
        Alignment::Aligned {
            basis: AlignmentBasis::EngineFrame { epoch },
            offset,
            overlap,
            ..
        } => {
            assert_eq!(epoch.get(), EVAL_EPOCH);
            assert_eq!(*offset, 0, "engine basis is already signal-aligned");
            assert_eq!(
                *overlap,
                Some(FrameInterval::new(0, TOTAL as u64)),
                "full fixed-input window jointly observed"
            );
        }
        other => panic!("expected Aligned EngineFrame, got {other:?}"),
    }

    // Per-candidate latency knowledge is carried on the records —
    // the consumer can compensate candidate B's declared latency
    // without Ruviform guessing.
    assert_eq!(sa.latency, LatencyKnowledge::Known { frames: 0 });
    assert_eq!(sb.latency, LatencyKnowledge::Known { frames: 32 });
}
