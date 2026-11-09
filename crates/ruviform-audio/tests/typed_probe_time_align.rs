//! Issue #309 PROBE-TIME fixture-slice tests — multi-probe time
//! alignment verified on a real `RtRuntime` + real delay DSP + a
//! deterministic audio fixture. Drained `MetricRecord`/
//! `CaptureChunk` streams are reconstructed into `ObservationSpan`/
//! `ObservationCoverage` and checked against `align_observation_points`
//! expectations computed independently of the implementation.
//!
//! Scope: `SharedCursor` basis only — `EngineFrame` anchor evidence
//! is a later slice (see `docs/designs/issue-309-probe-time-align-fixture.md`).
mod common;

use std::collections::BTreeMap;
use std::ops::Range;

use assert_no_alloc::assert_no_alloc;
use common::*;

use ruviform_audio::typed::probe::{
    AlignPolicy, Alignment, AlignmentBasis, AlignmentCondition, CaptureConfig, CoverageClass,
    CoverageEvent, FrameInterval, GenerationRange, IncomparableReason, LatencyKnowledge,
    MetricRecord, ObservationCoverage, ObservationFormat, ObservationSpan, ProbeBroker,
    ProbeClockMark, ProbeCollector, ProbeConfig, ProbeCounts, ProbeEvent, ProbeId, ProbeMode,
    ProbePoint, ProbePrecision, RecordMeta, StatusRecord, align_observation_points, pair,
};
use ruviform_audio::typed::{Compiler, GraphProcessor, GraphRuntime, NodeIo, RtRuntime};
use ruviform_audio_core::ChannelLayout;
use ruviform_graph::{Direction, GraphBuilder, GraphSpec, NodeId, NodeKind, PortId, PortRole};
use ruviform_media_core::{ClockDomainId, ClockEpochId};
use ruviform_rt::RtContext;

const FRAMES: usize = 64;
const BLOCKS: usize = 8;
const TOTAL_FRAMES: usize = FRAMES * BLOCKS;
/// Per-processor delay — block-aligned so every latency difference is
/// a whole number of `sample_position` blocks.
const LA: usize = FRAMES;
const LB: usize = FRAMES;
/// `ProbeConfig::chunk_frames` for capture fixtures — one block per
/// chunk, keeping `CaptureChunk.data` indexing trivial.
const CHUNK_FRAMES: usize = FRAMES;
/// `CompilerConfig::max_channels` — `DelayDsp` preallocates this many
/// delay-line planes (construction-time only, RT path never grows).
const DSP_CHANNELS: usize = 8;

// ---- fixture graph: Source → DelayDsp(LA) → DelayDsp(LB) → Sink ----

struct DelayChain {
    spec: GraphSpec,
    src_out: PortId,
    d1: NodeId,
    d1_in: PortId,
    d1_out: PortId,
    d2: NodeId,
    sink: NodeId,
    sink_in: PortId,
}

fn delay_chain() -> DelayChain {
    let mut b = GraphBuilder::new();
    let (src, sp) = b.add_node(&node(
        NodeKind::Source,
        vec![port_probe(Direction::Output, PortRole::Main, stereo())],
    ));
    let (d1, d1p) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port_probe(Direction::Input, PortRole::Main, stereo()),
            port_probe(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (d2, d2p) = b.add_node(&node(
        NodeKind::Processor,
        vec![
            port_probe(Direction::Input, PortRole::Main, stereo()),
            port_probe(Direction::Output, PortRole::Main, stereo()),
        ],
    ));
    let (sink, kp) = b.add_node(&node(
        NodeKind::Sink,
        vec![port_probe(Direction::Input, PortRole::Main, stereo())],
    ));
    b.connect(&conn(pref(src, sp[0]), pref(d1, d1p[0])));
    b.connect(&conn(pref(d1, d1p[1]), pref(d2, d2p[0])));
    b.connect(&conn(pref(d2, d2p[1]), pref(sink, kp[0])));
    DelayChain {
        spec: b.build(),
        src_out: sp[0],
        d1,
        d1_in: d1p[0],
        d1_out: d1p[1],
        d2,
        sink,
        sink_in: kp[0],
    }
}

/// Real delay-line processor — `latency()` reports `D` and `process`
/// genuinely delays every channel by `D` frames (zeros before the
/// first input), so `RecordMeta.effective_latency` matches the
/// observed PCM timing. `process` uses only `copy_within`/
/// `copy_from_slice` — RT no-alloc.
struct DelayDsp {
    latency: usize,
    /// Per-channel input history, `latency` frames each
    /// (oldest → newest).
    lines: Vec<Vec<f32>>,
}

impl DelayDsp {
    fn new(latency: usize) -> Self {
        Self {
            latency,
            lines: vec![vec![0.0; latency]; DSP_CHANNELS],
        }
    }
}

impl GraphProcessor for DelayDsp {
    fn latency(&self) -> u32 {
        u32::try_from(self.latency).unwrap_or(u32::MAX)
    }

    fn activate(&mut self, _ctx: &RtContext) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let input = io.input(0);
        let latency = self.latency;
        for o in 0..io.outputs() {
            let Some(mut out) = io.output(o) else {
                continue;
            };
            for c in 0..out.channels() {
                let (Some(plane), Some(line)) = (out.channel_mut(c), self.lines.get_mut(c)) else {
                    continue;
                };
                let n = plane.len();
                let src = input.as_ref().and_then(|v| v.channel(c));
                for i in 0..n {
                    plane[i] = if i < latency {
                        line[i]
                    } else {
                        src.and_then(|s| s.get(i - latency)).copied().unwrap_or(0.0)
                    };
                }
                // Advance the line to the last `latency` inputs.
                if n >= latency {
                    match src.and_then(|s| s.get(n - latency..n)) {
                        Some(tail) => line.copy_from_slice(tail),
                        None => line.fill(0.0),
                    }
                } else {
                    line.copy_within(n..latency, 0);
                    let head = src.map_or(&[][..], |s| &s[..n.min(s.len())]);
                    let dst = &mut line[latency - n..];
                    let take = head.len().min(n);
                    dst[..take].copy_from_slice(&head[..take]);
                    dst[take..].fill(0.0);
                }
            }
        }
    }
}

// ---- fixture signal ----

/// Whether block `k` feeds measured silence (valid coverage, zero
/// level — never "missing" data).
fn is_silent_block(k: usize) -> bool {
    (5..7).contains(&k)
}

/// Deterministic stereo fixture: per-channel sine at distinct
/// frequencies with a per-block amplitude ramp, blocks 5–6 silent.
/// Returned as channel × absolute-frame planes so expectations can
/// be computed independently of the runtime.
#[expect(
    clippy::cast_precision_loss,
    reason = "fixture signal generation — f32 fixture math"
)]
fn fixture_input() -> [Vec<f32>; 2] {
    let mut ch0 = vec![0.0_f32; TOTAL_FRAMES];
    let mut ch1 = vec![0.0_f32; TOTAL_FRAMES];
    for k in 0..BLOCKS {
        if is_silent_block(k) {
            continue;
        }
        let a0 = 0.4 + 0.05 * k as f32;
        let a1 = 0.3 + 0.04 * k as f32;
        for i in 0..FRAMES {
            let t = k * FRAMES + i;
            ch0[t] = a0 * (core::f32::consts::TAU * t as f32 / 48.0).sin();
            ch1[t] = a1 * (core::f32::consts::TAU * t as f32 / 32.0 + 0.7).sin();
        }
    }
    [ch0, ch1]
}

// ---- rig / run helpers ----

struct Rig {
    grt: GraphRuntime,
    rt: RtRuntime,
    broker: ProbeBroker,
    collector: ProbeCollector,
}

fn rig(chain: &DelayChain, probe_config: &ProbeConfig, factory: &mut MockFactory) -> Rig {
    let cfg = config(FRAMES);
    let compiler = Compiler::new(cfg.clone());
    let (mut grt, mut rt) = GraphRuntime::new(&cfg, 8, 8);
    let plan_out = compiler
        .compile(
            &validate(&chain.spec),
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
        broker: parts.broker,
        collector: parts.collector,
    }
}

fn delay_factory(chain: &DelayChain, reuse: bool) -> MockFactory {
    let mut f = MockFactory {
        reuse,
        ..MockFactory::default()
    };
    f.pending.insert(chain.d1, Box::new(DelayDsp::new(LA)));
    f.pending.insert(chain.d2, Box::new(DelayDsp::new(LB)));
    f
}

#[derive(Copy, Clone)]
struct ProbeIds {
    p0: ProbeId,
    p1: ProbeId,
    p2: ProbeId,
    pt0: ProbePoint,
    pt1: ProbePoint,
    pt2: ProbePoint,
}

/// Installs probes at latency 0 (d1 input), LA (d1 output) and
/// LA+LB (sink input = graph output).
fn install_three(rig: &mut Rig, chain: &DelayChain, mode: ProbeMode) -> ProbeIds {
    let pt0 = ProbePoint {
        node: chain.d1,
        port: chain.d1_in,
    };
    let pt1 = ProbePoint {
        node: chain.d1,
        port: chain.d1_out,
    };
    let pt2 = ProbePoint {
        node: chain.sink,
        port: chain.sink_in,
    };
    let p0 = rig
        .broker
        .install(&rig.grt, &chain.spec, pt0, mode)
        .unwrap();
    let p1 = rig
        .broker
        .install(&rig.grt, &chain.spec, pt1, mode)
        .unwrap();
    let p2 = rig
        .broker
        .install(&rig.grt, &chain.spec, pt2, mode)
        .unwrap();
    ProbeIds {
        p0,
        p1,
        p2,
        pt0,
        pt1,
        pt2,
    }
}

fn ctx() -> RtContext {
    RtContext::mint()
}

fn run_blocks(rig: &mut Rig, chain: &DelayChain, input: &[Vec<f32>; 2], range: Range<usize>) {
    for k in range {
        let s = k * FRAMES;
        let l = &input[0][s..s + FRAMES];
        let r = &input[1][s..s + FRAMES];
        feed(&mut rig.rt, chain.src_out, &[l, r]);
        rig.rt.process(FRAMES, &ctx()).expect("process");
    }
}

/// Republish + rebind against a new plan generation (Issue #308
/// path) — processors are reused so DSP state continues.
fn republish(rig: &mut Rig, chain: &DelayChain, factory: &mut MockFactory) {
    let cfg = config(FRAMES);
    let compiler = Compiler::new(cfg);
    let plan_out = compiler
        .compile(
            &validate(&chain.spec),
            factory,
            rig.grt.registry(),
            rig.grt.allocator(),
            rig.grt.generation() + 1,
        )
        .expect("recompile");
    rig.grt.publish(plan_out).expect("republish");
    rig.broker
        .on_publish(&rig.grt, &chain.spec)
        .expect("rebind");
}

/// Owned probe event — PCM copied out of the pooled chunk so views
/// outlive the drain callback.
#[derive(Clone, Debug)]
enum OwnedEv {
    Metric(MetricRecord),
    Status(StatusRecord),
    Cap {
        meta: RecordMeta,
        /// Per-channel planes, `frames` samples each.
        planes: Vec<Vec<f32>>,
    },
}

fn drain_into(rig: &mut Rig, out: &mut BTreeMap<ProbeId, Vec<OwnedEv>>) {
    rig.collector.drain(|ev| {
        let owned = match ev {
            ProbeEvent::Metric(m) => Some((m.meta.probe, OwnedEv::Metric(m))),
            ProbeEvent::Status(s) => Some((s.probe, OwnedEv::Status(s))),
            ProbeEvent::Captured(c) => {
                let meta = c.meta.expect("captured chunk carries meta");
                let mut planes = Vec::with_capacity(c.channels);
                for ch in 0..c.channels {
                    let base = ch * CHUNK_FRAMES;
                    planes.push(c.data[base..base + c.frames].to_vec());
                }
                Some((meta.probe, OwnedEv::Cap { meta, planes }))
            }
            _ => None,
        };
        if let Some((probe, ev)) = owned {
            out.entry(probe).or_default().push(ev);
        }
    });
}

/// Per-probe facts reconstructed from drained records — owns the
/// coverage so [`ProbeView::span`] can lend it to `ObservationSpan`.
struct ProbeView {
    coverage: ObservationCoverage,
    interval: FrameInterval,
    generation_range: GenerationRange,
    format: ObservationFormat,
    latency: LatencyKnowledge,
    clock_mark: ProbeClockMark,
}

impl ProbeView {
    fn span(&self, probe: ProbeId, point: ProbePoint) -> ObservationSpan<'_> {
        ObservationSpan {
            probe,
            point,
            clock_mark: self.clock_mark,
            interval: self.interval,
            generation_range: self.generation_range,
            format: self.format,
            latency: self.latency,
            coverage: &self.coverage,
            context: None,
            precision: ProbePrecision::F32Rt,
        }
    }
}

/// Rebuilds coverage + span facts for `probe` from its drained
/// events. `lost_records` is the telemetry-loss counter delta over
/// the drained window. `clock_mark`/`latency`/`format` come from the
/// first data record — mixed-mark windows surface through coverage
/// interval marks instead (see the epoch-discontinuity test).
fn build_view(
    probe: ProbeId,
    evs: &[OwnedEv],
    lost_records: u64,
    layout: ChannelLayout,
) -> ProbeView {
    let coverage = ObservationCoverage::reconstruct(
        probe,
        lost_records,
        evs.iter().map(|e| match *e {
            OwnedEv::Metric(m) => CoverageEvent::Data(m.meta),
            OwnedEv::Status(s) => CoverageEvent::Status(s),
            OwnedEv::Cap { meta, .. } => CoverageEvent::Data(meta),
        }),
    );
    let mut first_meta: Option<RecordMeta> = None;
    let mut min_pos = u64::MAX;
    let mut max_end = 0_u64;
    let mut gen_first = u64::MAX;
    let mut gen_last = 0_u64;
    for e in evs {
        let (data_meta, generation) = match *e {
            OwnedEv::Metric(m) => (Some(m.meta), m.meta.generation),
            OwnedEv::Cap { meta, .. } => (Some(meta), meta.generation),
            OwnedEv::Status(s) => (None, s.generation),
        };
        gen_first = gen_first.min(generation);
        gen_last = gen_last.max(generation);
        if let Some(m) = data_meta {
            if first_meta.is_none() {
                first_meta = Some(m);
            }
            min_pos = min_pos.min(m.sample_position);
            max_end = max_end.max(m.sample_position.saturating_add(u64::from(m.frames)));
        }
    }
    let interval = if min_pos == u64::MAX {
        FrameInterval::new(0, 0)
    } else {
        FrameInterval::new(min_pos, max_end)
    };
    let generation_range = if gen_first == u64::MAX {
        GenerationRange::single(0)
    } else {
        GenerationRange {
            first: gen_first,
            last: gen_last,
        }
    };
    let (format, latency, clock_mark) = match first_meta {
        Some(m) => (
            ObservationFormat {
                channels: m.channels,
                sample_rate: m.sample_rate,
                channel_layout: layout,
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
                channel_layout: layout,
            },
            LatencyKnowledge::Unknown,
            ProbeClockMark::UNKNOWN,
        ),
    };
    ProbeView {
        coverage,
        interval,
        generation_range,
        format,
        latency,
        clock_mark,
    }
}

fn build_views(
    evs: &BTreeMap<ProbeId, Vec<OwnedEv>>,
    lost_records: u64,
) -> BTreeMap<ProbeId, ProbeView> {
    evs.iter()
        .map(|(&pid, ev)| {
            (
                pid,
                build_view(pid, ev, lost_records, ChannelLayout::Stereo),
            )
        })
        .collect()
}

// ---- record accessors / independent expectations ----

fn metric_at(evs: &[OwnedEv], pos: u64) -> Option<MetricRecord> {
    evs.iter().find_map(|e| match e {
        OwnedEv::Metric(m) if m.meta.sample_position == pos => Some(*m),
        _ => None,
    })
}

fn chunk_at(evs: &[OwnedEv], pos: u64) -> Option<&[Vec<f32>]> {
    evs.iter().find_map(|e| match e {
        OwnedEv::Cap { meta, planes, .. } if meta.sample_position == pos => Some(planes.as_slice()),
        _ => None,
    })
}

fn class_at(cov: &ObservationCoverage, pos: u64) -> Option<CoverageClass> {
    cov.intervals
        .iter()
        .find(|iv| iv.start <= pos && pos < iv.end)
        .map(|iv| iv.class)
}

/// Peak/RMS of `input[start..start+frames)` in f64 — positions
/// before frame 0 read as zeros (the delay line's pre-roll).
#[expect(
    clippy::cast_precision_loss,
    reason = "independent f64 expectation over the f32 fixture"
)]
fn expected_level(input: &[f32], start: i64, frames: usize) -> (f64, f64) {
    let mut peak = 0.0_f64;
    let mut sum = 0.0_f64;
    for i in 0..frames {
        let pos = start + i64::try_from(i).unwrap_or(i64::MAX);
        let x = usize::try_from(pos)
            .ok()
            .and_then(|p| input.get(p))
            .copied()
            .unwrap_or(0.0);
        let v = f64::from(x);
        peak = peak.max(v.abs());
        sum += v * v;
    }
    (peak, (sum / frames as f64).sqrt())
}

fn assert_planes_eq(first: &[Vec<f32>], second: &[Vec<f32>], what: &str) {
    assert_eq!(first.len(), second.len(), "{what}: channel count");
    for (ch, (pa, pb)) in first.iter().zip(second).enumerate() {
        assert_eq!(pa.len(), pb.len(), "{what}: ch{ch} frame count");
        for (i, (x, y)) in pa.iter().zip(pb).enumerate() {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "{what}: ch{ch} frame {i}: {x} vs {y}"
            );
        }
    }
}

// ---- alignment result accessors ----

fn expect_aligned(al: &Alignment) -> (AlignmentBasis, i64, u64, Option<FrameInterval>) {
    match al {
        Alignment::Aligned {
            basis,
            offset,
            error_budget,
            overlap,
        } => (*basis, *offset, *error_budget, *overlap),
        other => panic!("expected Aligned, got {other:?}"),
    }
}

fn expect_conditional(al: &Alignment) -> (i64, Vec<AlignmentCondition>, Vec<FrameInterval>) {
    match al {
        Alignment::Conditional {
            offset,
            conditions,
            excluded_intervals,
            ..
        } => (*offset, conditions.clone(), excluded_intervals.clone()),
        other => panic!("expected Conditional, got {other:?}"),
    }
}

fn expect_incomparable(al: &Alignment) -> IncomparableReason {
    match al {
        Alignment::Incomparable { reason } => *reason,
        other => panic!("expected Incomparable, got {other:?}"),
    }
}

// ---- shared run builders ----

/// Monitor-mode run: 3 probes, 8 blocks, single end drain.
struct MonitorRun {
    rig: Rig,
    ids: ProbeIds,
    evs: BTreeMap<ProbeId, Vec<OwnedEv>>,
    input: [Vec<f32>; 2],
}

fn monitor_run() -> MonitorRun {
    let chain = delay_chain();
    let input = fixture_input();
    let mut factory = delay_factory(&chain, false);
    let mut rig = rig(&chain, &ProbeConfig::default(), &mut factory);
    let ids = install_three(&mut rig, &chain, ProbeMode::Monitor);
    rig.broker.wire_clock_mark().unwrap();
    run_blocks(&mut rig, &chain, &input, 0..BLOCKS);
    let mut evs = BTreeMap::new();
    drain_into(&mut rig, &mut evs);
    MonitorRun {
        rig,
        ids,
        evs,
        input,
    }
}

/// Starts a rig with 3 Monitor probes + wired clock mark.
fn start_monitor(chain: &DelayChain, probe_config: &ProbeConfig, reuse: bool) -> (Rig, ProbeIds) {
    let mut factory = delay_factory(chain, reuse);
    let mut rig = rig(chain, probe_config, &mut factory);
    let ids = install_three(&mut rig, chain, ProbeMode::Monitor);
    rig.broker.wire_clock_mark().unwrap();
    (rig, ids)
}

fn assert_counts_clean(counts: &ProbeCounts) {
    assert_eq!(counts.lost_records, 0, "unexpected telemetry loss");
    assert_eq!(counts.lost_blocks, 0, "unexpected chunk loss");
    assert_eq!(counts.unobserved, 0, "unexpected unobserved blocks");
    assert_eq!(counts.unbound, 0, "unexpected unbound blocks");
}

// ---- tests ----

/// Three probes at effective latencies 0 / LA / LA+LB align pairwise
/// on `SharedCursor` with offsets equal to the latency differences and
/// overlaps equal to the latency-shifted joint windows.
#[test]
fn monitor_align_reports_latency_offsets() {
    let run = monitor_run();
    assert_counts_clean(&run.rig.collector.counts());
    let lost = run.rig.collector.counts().lost_records;
    let views = build_views(&run.evs, lost);
    let policy = AlignPolicy::default();
    let mark = run.rig.broker.time_context().mark();

    // Record-carried latencies are the fixture ground truth.
    for (pid, want) in [
        (run.ids.p0, 0_u32),
        (run.ids.p1, u32::try_from(LA).unwrap()),
        (run.ids.p2, u32::try_from(LA + LB).unwrap()),
    ] {
        let evs = run.evs.get(&pid).expect("probe events");
        let lats: Vec<u32> = evs
            .iter()
            .filter_map(|e| match e {
                OwnedEv::Metric(m) => Some(m.meta.effective_latency),
                OwnedEv::Cap { meta, .. } => Some(meta.effective_latency),
                OwnedEv::Status(_) => None,
            })
            .collect();
        assert!(!lats.is_empty(), "probe {pid:?} emitted no data records");
        assert!(
            lats.iter().all(|&l| l == want),
            "probe {pid:?} latency {lats:?} != {want}"
        );
    }

    let span_early = views[&run.ids.p0].span(run.ids.p0, run.ids.pt0);
    let span_mid = views[&run.ids.p1].span(run.ids.p1, run.ids.pt1);
    let span_late = views[&run.ids.p2].span(run.ids.p2, run.ids.pt2);
    let basis = AlignmentBasis::SharedCursor {
        domain: mark.domain,
        epoch: mark.epoch,
    };

    // p0 (latency 0) → p1 (latency LA): offset = LA, overlap [LA, 512).
    let (b, offset, budget, overlap) =
        expect_aligned(&align_observation_points(&span_early, &span_mid, &policy));
    assert_eq!(b, basis);
    assert_eq!(offset, i64::try_from(LA).unwrap());
    assert_eq!(budget, 0);
    assert_eq!(overlap, Some(FrameInterval::new(64, 512)));

    // p1 → p2: offset = LB, same overlap shape.
    let (b, offset, budget, overlap) =
        expect_aligned(&align_observation_points(&span_mid, &span_late, &policy));
    assert_eq!(b, basis);
    assert_eq!(offset, i64::try_from(LB).unwrap());
    assert_eq!(budget, 0);
    assert_eq!(overlap, Some(FrameInterval::new(64, 512)));

    // p0 → p2: offset = LA + LB.
    let (b, offset, budget, overlap) =
        expect_aligned(&align_observation_points(&span_early, &span_late, &policy));
    assert_eq!(b, basis);
    assert_eq!(offset, i64::try_from(LA + LB).unwrap());
    assert_eq!(budget, 0);
    assert_eq!(overlap, Some(FrameInterval::new(128, 512)));

    // Span facts reconstructed purely from records.
    for view in views.values() {
        assert_eq!(view.interval, FrameInterval::new(0, 512));
        assert_eq!(view.generation_range, GenerationRange::single(1));
        assert_eq!(view.format.channels, 2);
        assert_eq!(view.format.sample_rate, 48_000);
        assert_eq!(view.format.channel_layout, ChannelLayout::Stereo);
    }
}

/// Per-block Peak/RMS equals an independent f64 recomputation over
/// the fixture input shifted by each record's own `effective_latency`;
/// offset-aligned probes agree bit-exactly, and measured silence stays
/// `Valid` with zero level.
#[test]
fn monitor_metric_levels_match_independent_fixture() {
    let run = monitor_run();
    let lost = run.rig.collector.counts().lost_records;
    let views = build_views(&run.evs, lost);

    for &pid in &[run.ids.p0, run.ids.p1, run.ids.p2] {
        let evs = run.evs.get(&pid).expect("probe events");
        let view = &views[&pid];
        for k in 0..BLOCKS {
            let pos = u64::try_from(k * FRAMES).unwrap();
            let m = metric_at(evs, pos).unwrap_or_else(|| panic!("metric at {pos}"));
            for ch in 0..2 {
                let start = i64::try_from(pos).unwrap() - i64::from(m.meta.effective_latency);
                let (peak, rms) = expected_level(&run.input[ch], start, FRAMES);
                assert!(
                    (f64::from(m.levels[ch].peak) - peak).abs() < 1e-6,
                    "probe {pid:?} ch{ch} pos {pos}: peak {} vs expected {peak}",
                    m.levels[ch].peak
                );
                assert!(
                    (f64::from(m.levels[ch].rms) - rms).abs() < 1e-3,
                    "probe {pid:?} ch{ch} pos {pos}: rms {} vs expected {rms}",
                    m.levels[ch].rms
                );
            }
            // Measured silence is derived from the independent
            // expectation over the latency-shifted window: wherever
            // the observed signal is exactly zero the probe must
            // measure zero AND keep `Valid` coverage — measured
            // silence is never collapsed into missing/unobserved.
            let silent = (0..2).all(|ch| {
                let start = i64::try_from(pos).unwrap() - i64::from(m.meta.effective_latency);
                expected_level(&run.input[ch], start, FRAMES).0 == 0.0
            });
            if silent {
                for level in &m.levels[..2] {
                    assert_eq!(level.peak.to_bits(), 0, "silent peak nonzero");
                    assert_eq!(level.rms.to_bits(), 0, "silent rms nonzero");
                }
                assert_eq!(
                    class_at(&view.coverage, pos),
                    Some(CoverageClass::Valid),
                    "silent block must stay Valid coverage"
                );
            }
        }
    }

    // Offset-aligned probes observe identical samples → identical
    // f32 measurements (bit-exact).
    let evs_early = run.evs.get(&run.ids.p0).unwrap();
    let evs_late = run.evs.get(&run.ids.p2).unwrap();
    for k in 0..BLOCKS - 2 {
        let m_early = metric_at(evs_early, u64::try_from(k * FRAMES).unwrap()).unwrap();
        let m_late = metric_at(evs_late, u64::try_from(k * FRAMES + LA + LB).unwrap()).unwrap();
        for ch in 0..2 {
            assert_eq!(
                m_early.levels[ch].peak.to_bits(),
                m_late.levels[ch].peak.to_bits(),
                "peak mismatch at offset pair k={k} ch={ch}"
            );
            assert_eq!(
                m_early.levels[ch].rms.to_bits(),
                m_late.levels[ch].rms.to_bits(),
                "rms mismatch at offset pair k={k} ch={ch}"
            );
        }
    }
}

/// Capture-mode run: 3 probes, per-block drain so the chunk pool
/// never exhausts; `process` stays alloc-free with capture active.
struct CaptureRun {
    rig: Rig,
    ids: ProbeIds,
    evs: BTreeMap<ProbeId, Vec<OwnedEv>>,
    input: [Vec<f32>; 2],
}

fn capture_run() -> CaptureRun {
    let chain = delay_chain();
    let input = fixture_input();
    let probe_config = ProbeConfig {
        chunk_frames: CHUNK_FRAMES,
        ..ProbeConfig::default()
    };
    let mut factory = delay_factory(&chain, false);
    let mut rig = rig(&chain, &probe_config, &mut factory);
    let ids = install_three(&mut rig, &chain, ProbeMode::Monitor);
    for &pid in &[ids.p0, ids.p1, ids.p2] {
        rig.broker
            .capture(
                pid,
                CaptureConfig {
                    max_frames: u64::MAX,
                },
            )
            .unwrap();
    }
    rig.broker.wire_clock_mark().unwrap();
    let mut evs = BTreeMap::new();
    for k in 0..BLOCKS {
        let s = k * FRAMES;
        let l = &input[0][s..s + FRAMES];
        let r = &input[1][s..s + FRAMES];
        feed(&mut rig.rt, chain.src_out, &[l, r]);
        assert_no_alloc(|| rig.rt.process(FRAMES, &ctx()).unwrap());
        drain_into(&mut rig, &mut evs);
    }
    CaptureRun {
        rig,
        ids,
        evs,
        input,
    }
}

/// Captured PCM aligns bit-exactly under the latency offset, and
/// PCM-derived levels over the jointly valid window match the
/// independent fixture expectation.
#[test]
fn capture_pcm_aligns_bit_exact_with_offsets() {
    let run = capture_run();
    assert_counts_clean(&run.rig.collector.counts());
    let lost = run.rig.collector.counts().lost_records;
    let views = build_views(&run.evs, lost);
    let policy = AlignPolicy::default();

    let span_early = views[&run.ids.p0].span(run.ids.p0, run.ids.pt0);
    let span_late = views[&run.ids.p2].span(run.ids.p2, run.ids.pt2);
    let (basis, offset, budget, overlap) =
        expect_aligned(&align_observation_points(&span_early, &span_late, &policy));
    assert!(matches!(basis, AlignmentBasis::SharedCursor { .. }));
    assert_eq!(offset, i64::try_from(LA + LB).unwrap());
    assert_eq!(budget, 0);
    assert_eq!(overlap, Some(FrameInterval::new(128, 512)));

    // Bit-exact PCM pairs across the whole joint-valid window:
    // p0 block p ↔ p2 block p + (LA+LB).
    let evs_early = run.evs.get(&run.ids.p0).unwrap();
    let evs_late = run.evs.get(&run.ids.p2).unwrap();
    for k in 0..BLOCKS - 2 {
        let pos_early = u64::try_from(k * FRAMES).unwrap();
        let pos_late = u64::try_from(k * FRAMES + LA + LB).unwrap();
        let first =
            chunk_at(evs_early, pos_early).unwrap_or_else(|| panic!("p0 chunk at {pos_early}"));
        let second =
            chunk_at(evs_late, pos_late).unwrap_or_else(|| panic!("p2 chunk at {pos_late}"));
        assert_planes_eq(first, second, "p0↔p2 pcm");
    }

    // PCM-derived peak/RMS over the compared window equals the
    // independent f64 expectation over input[0..384).
    for ch in 0..2 {
        let (mut peak, mut sum) = (0.0_f64, 0.0_f64);
        let mut n = 0_usize;
        for k in 0..BLOCKS - 2 {
            let planes = chunk_at(evs_early, u64::try_from(k * FRAMES).unwrap()).unwrap();
            for &x in &planes[ch] {
                let v = f64::from(x);
                peak = peak.max(v.abs());
                sum += v * v;
                n += 1;
            }
        }
        let (want_peak, want_rms) = expected_level(&run.input[ch], 0, (BLOCKS - 2) * FRAMES);
        assert!((peak - want_peak).abs() < 1e-9);
        let n_f = f64::from(u32::try_from(n).unwrap());
        assert!(((sum / n_f).sqrt() - want_rms).abs() < 1e-9);
    }
}

/// `test-rt` lane (`rt_` name filter): the full delay chain with
/// three Capture probes processes every block alloc-free — pooled
/// chunks keep capture inside the RT contract. `capture_run`
/// wraps each `process` in `assert_no_alloc`.
#[test]
fn rt_capture_process_allocates_nothing() {
    let run = capture_run();
    // Chunks prove the taps were live inside the no-alloc windows.
    assert!(!run.evs[&run.ids.p0].is_empty());
    assert!(!run.evs[&run.ids.p2].is_empty());
}

/// Probe OFF is a declared observation gap: alignment downgrades to
/// `Conditional{PartialCoverage}` with the Off window excluded in
/// B-side coordinates, while measured silence stays comparable.
#[test]
fn probe_off_window_is_conditional_and_masked() {
    let chain = delay_chain();
    let input = fixture_input();
    let (mut rig, ids) = start_monitor(&chain, &ProbeConfig::default(), false);
    run_blocks(&mut rig, &chain, &input, 0..2);
    rig.broker.set_mode(ids.p2, ProbeMode::Off).unwrap();
    run_blocks(&mut rig, &chain, &input, 2..4);
    rig.broker.set_mode(ids.p2, ProbeMode::Monitor).unwrap();
    run_blocks(&mut rig, &chain, &input, 4..BLOCKS);
    let mut evs = BTreeMap::new();
    drain_into(&mut rig, &mut evs);
    let lost = rig.collector.counts().lost_records;
    assert_counts_clean(&rig.collector.counts());
    let views = build_views(&evs, lost);
    let policy = AlignPolicy::default();

    // p2 emitted nothing during cursor blocks 2–3 ([128, 256)).
    let evs_late = evs.get(&ids.p2).unwrap();
    assert!(metric_at(evs_late, 128).is_none());
    assert!(metric_at(evs_late, 192).is_none());
    assert!(metric_at(evs_late, 256).is_some());

    let view_early = &views[&ids.p0];
    let view_late = &views[&ids.p2];
    // Off state classified as Off; surrounding data stays Valid —
    // including the silent blocks (measured silence ≠ gap).
    assert_eq!(class_at(&view_late.coverage, 160), Some(CoverageClass::Off));
    assert_eq!(
        class_at(&view_late.coverage, 320),
        Some(CoverageClass::Valid)
    );

    let span_early = view_early.span(ids.p0, ids.pt0);
    let span_late = view_late.span(ids.p2, ids.pt2);
    let (offset, conditions, excluded) =
        expect_conditional(&align_observation_points(&span_early, &span_late, &policy));
    assert_eq!(offset, i64::try_from(LA + LB).unwrap());
    assert!(conditions.contains(&AlignmentCondition::PartialCoverage));
    assert_eq!(excluded, vec![FrameInterval::new(128, 256)]);

    // The silent region ([320, 448) in B coords) is NOT excluded —
    // its zero-level metrics still compare equal across probes.
    assert!(
        !excluded.iter().any(|iv| iv.start < 448 && iv.end > 320),
        "measured silence must not be masked"
    );
    let m_early = metric_at(evs.get(&ids.p0).unwrap(), 320).unwrap();
    let m_late = metric_at(evs_late, 448).unwrap();
    for ch in 0..2 {
        assert_eq!(m_early.levels[ch].peak.to_bits(), 0);
        assert_eq!(m_late.levels[ch].peak.to_bits(), 0);
    }

    // Disallowing conditional results turns the same evidence into
    // an explicit Incomparable.
    let strict = AlignPolicy {
        allow_conditional: false,
        ..AlignPolicy::default()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(&span_early, &span_late, &strict)),
        IncomparableReason::DiscontinuityInWindow
    );
}

/// Chunk-pool exhaustion drops PCM with a `LostBlocks` status —
/// the loss lands in coverage and `excluded_intervals` in *signal*
/// coordinates on both sides, never as continuous PCM.
#[test]
fn lost_blocks_are_masked_from_pcm_comparison() {
    let chain = delay_chain();
    let input = fixture_input();
    let probe_config = ProbeConfig {
        pool_chunks: 2,
        chunk_frames: CHUNK_FRAMES,
        ..ProbeConfig::default()
    };
    let mut factory = delay_factory(&chain, false);
    let mut rig = rig(&chain, &probe_config, &mut factory);
    // Two capture probes fill the 2-chunk pool every block.
    let pt0 = ProbePoint {
        node: chain.d1,
        port: chain.d1_in,
    };
    let pt2 = ProbePoint {
        node: chain.sink,
        port: chain.sink_in,
    };
    let p0 = rig
        .broker
        .install(&rig.grt, &chain.spec, pt0, ProbeMode::Monitor)
        .unwrap();
    let p2 = rig
        .broker
        .install(&rig.grt, &chain.spec, pt2, ProbeMode::Monitor)
        .unwrap();
    for &pid in &[p0, p2] {
        rig.broker
            .capture(
                pid,
                CaptureConfig {
                    max_frames: u64::MAX,
                },
            )
            .unwrap();
    }
    rig.broker.wire_clock_mark().unwrap();
    let mut evs: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();

    // Blocks 0–2 drain each block; block 3 leaves 2 chunks
    // outstanding; block 4 finds the pool empty → LostBlocks.
    for k in 0..3 {
        run_blocks(&mut rig, &chain, &input, k..k + 1);
        drain_into(&mut rig, &mut evs);
    }
    run_blocks(&mut rig, &chain, &input, 3..4);
    run_blocks(&mut rig, &chain, &input, 4..5);
    drain_into(&mut rig, &mut evs);
    for k in 5..BLOCKS {
        run_blocks(&mut rig, &chain, &input, k..k + 1);
        drain_into(&mut rig, &mut evs);
    }

    let counts = rig.collector.counts();
    assert_eq!(counts.lost_blocks, 2, "one lost block per probe");
    assert_eq!(counts.lost_records, 0);
    let lost = counts.lost_records;
    let views = build_views(&evs, lost);
    let policy = AlignPolicy::default();

    let view_early = &views[&p0];
    let view_late = &views[&p2];
    assert_eq!(
        class_at(&view_early.coverage, 288),
        Some(CoverageClass::LostBlocks)
    );
    assert_eq!(
        class_at(&view_late.coverage, 288),
        Some(CoverageClass::LostBlocks)
    );
    // No chunk exists at the lost cursor position on either probe.
    assert!(chunk_at(evs.get(&p0).unwrap(), 256).is_none());
    assert!(chunk_at(evs.get(&p2).unwrap(), 256).is_none());

    let span_early = view_early.span(p0, pt0);
    let span_late = view_late.span(p2, pt2);
    let (offset, conditions, excluded) =
        expect_conditional(&align_observation_points(&span_early, &span_late, &policy));
    assert_eq!(offset, i64::try_from(LA + LB).unwrap());
    assert!(conditions.contains(&AlignmentCondition::PartialCoverage));
    // The cursor loss at [256,320) masks *two* signal windows in
    // B-side coordinates: B's own lost block [256,320) and A's
    // lost block shifted by the offset → [384,448).
    assert_eq!(
        excluded,
        vec![FrameInterval::new(256, 320), FrameInterval::new(384, 448),]
    );

    // Only jointly-valid pairs carry PCM: p0 blocks {0,64,192,320}
    // ↔ p2 blocks {128,192,320,448}.
    let evs_early = evs.get(&p0).unwrap();
    let evs_late = evs.get(&p2).unwrap();
    for (a_pos, b_pos) in [(0, 128), (64, 192), (192, 320), (320, 448)] {
        let first = chunk_at(evs_early, a_pos).unwrap();
        let second = chunk_at(evs_late, b_pos).unwrap();
        assert_planes_eq(first, second, "loss-masked pcm pair");
    }
    // The masked cursor positions have no comparable counterpart.
    assert!(chunk_at(evs_early, 128).is_some());
    assert!(chunk_at(evs_late, 256).is_none());
    assert!(chunk_at(evs_early, 256).is_none());
    assert!(chunk_at(evs_late, 384).is_some());
}

/// `bump_epoch` (the shared representation of Seek/Loop/Device
/// Restart discontinuities) splits the record stream: cross-epoch
/// spans are `Incomparable{EpochMismatch}` and a span window
/// containing the boundary is `DiscontinuityInWindow`.
#[test]
fn epoch_bump_splits_incomparable_and_discontinuity() {
    let chain = delay_chain();
    let input = fixture_input();
    let (mut rig, ids) = start_monitor(&chain, &ProbeConfig::default(), false);
    run_blocks(&mut rig, &chain, &input, 0..4);
    let mut evs_a: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();
    drain_into(&mut rig, &mut evs_a);
    let epoch_a = rig.broker.bump_epoch().unwrap();
    run_blocks(&mut rig, &chain, &input, 4..BLOCKS);
    let mut evs_b: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();
    drain_into(&mut rig, &mut evs_b);

    // Records self-describe their epoch.
    for e in evs_a.get(&ids.p0).unwrap() {
        let clock = match e {
            OwnedEv::Metric(m) => m.meta.clock,
            OwnedEv::Status(s) => s.clock,
            OwnedEv::Cap { meta, .. } => meta.clock,
        };
        assert_eq!(clock.epoch, ClockEpochId::new(0));
    }
    for e in evs_b.get(&ids.p0).unwrap() {
        let clock = match e {
            OwnedEv::Metric(m) => m.meta.clock,
            OwnedEv::Status(s) => s.clock,
            OwnedEv::Cap { meta, .. } => meta.clock,
        };
        assert_eq!(clock.epoch, epoch_a);
    }

    let lost = rig.collector.counts().lost_records;
    let view_a = build_view(
        ids.p0,
        evs_a.get(&ids.p0).unwrap(),
        lost,
        ChannelLayout::Stereo,
    );
    let view_b = build_view(
        ids.p0,
        evs_b.get(&ids.p0).unwrap(),
        lost,
        ChannelLayout::Stereo,
    );
    let policy = AlignPolicy::default();
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &view_a.span(ids.p0, ids.pt0),
            &view_b.span(ids.p0, ids.pt0),
            &policy,
        )),
        IncomparableReason::EpochMismatch
    );

    // A span whose window contains the epoch boundary carries
    // mixed-mark coverage → hard discontinuity.
    let mut merged = evs_a.get(&ids.p0).unwrap().clone();
    merged.extend(evs_b.get(&ids.p0).unwrap().iter().cloned());
    let view_m = build_view(ids.p0, &merged, lost, ChannelLayout::Stereo);
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &view_m.span(ids.p0, ids.pt0),
            &view_m.span(ids.p0, ids.pt0),
            &policy,
        )),
        IncomparableReason::DiscontinuityInWindow
    );
}

/// A generation rebind (republish) is an observation discontinuity:
/// spans on different sides are `GenerationSpan`, a merged window
/// crosses generations, and only an explicit relaxed policy yields
/// `Conditional{CrossGeneration}`.
#[test]
fn generation_rebind_is_explicit_boundary() {
    let chain = delay_chain();
    let input = fixture_input();
    let (mut rig, ids) = start_monitor(&chain, &ProbeConfig::default(), true);
    run_blocks(&mut rig, &chain, &input, 0..4);
    let mut evs_g1: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();
    drain_into(&mut rig, &mut evs_g1);
    let mut factory = delay_factory(&chain, true);
    republish(&mut rig, &chain, &mut factory);
    run_blocks(&mut rig, &chain, &input, 4..BLOCKS);
    let mut evs_g2: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();
    drain_into(&mut rig, &mut evs_g2);

    // Records carry their own generation.
    for e in evs_g1.get(&ids.p0).unwrap() {
        let generation = match e {
            OwnedEv::Metric(m) => m.meta.generation,
            OwnedEv::Status(s) => s.generation,
            OwnedEv::Cap { meta, .. } => meta.generation,
        };
        assert_eq!(generation, 1);
    }
    for e in evs_g2.get(&ids.p0).unwrap() {
        let generation = match e {
            OwnedEv::Metric(m) => m.meta.generation,
            OwnedEv::Status(s) => s.generation,
            OwnedEv::Cap { meta, .. } => meta.generation,
        };
        assert_eq!(generation, 2);
    }

    let lost = rig.collector.counts().lost_records;
    let view_g1 = build_view(
        ids.p0,
        evs_g1.get(&ids.p0).unwrap(),
        lost,
        ChannelLayout::Stereo,
    );
    let view_g2 = build_view(
        ids.p0,
        evs_g2.get(&ids.p0).unwrap(),
        lost,
        ChannelLayout::Stereo,
    );
    let policy = AlignPolicy::default();

    // Different generation ranges → GenerationSpan even though the
    // cursor series itself is continuous.
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &view_g1.span(ids.p0, ids.pt0),
            &view_g2.span(ids.p0, ids.pt0),
            &policy,
        )),
        IncomparableReason::GenerationSpan
    );

    // A single span internally crossing the boundary also refuses.
    let mut merged = evs_g1.get(&ids.p0).unwrap().clone();
    merged.extend(evs_g2.get(&ids.p0).unwrap().iter().cloned());
    let view_m = build_view(ids.p0, &merged, lost, ChannelLayout::Stereo);
    assert!(view_m.generation_range.crosses());
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &view_m.span(ids.p0, ids.pt0),
            &view_m.span(ids.p0, ids.pt0),
            &policy,
        )),
        IncomparableReason::GenerationSpan
    );

    // Relaxed policy → Conditional{CrossGeneration} on the
    // overlapping portion (merged ∩ gen1 = [0,256)).
    let relaxed = AlignPolicy {
        require_same_generation: false,
        ..AlignPolicy::default()
    };
    let (offset, conditions, _) = expect_conditional(&align_observation_points(
        &view_m.span(ids.p0, ids.pt0),
        &view_g1.span(ids.p0, ids.pt0),
        &relaxed,
    ));
    assert_eq!(offset, 0);
    assert!(conditions.contains(&AlignmentCondition::CrossGeneration));
}

/// Single-field mutations of real record-derived spans exercise the
/// explicit Incomparable/Conditional boundaries: unknown latency,
/// format and rate.
#[test]
fn real_record_spans_drive_explicit_boundaries() {
    let run = monitor_run();
    let lost = run.rig.collector.counts().lost_records;
    let views = build_views(&run.evs, lost);
    let view_early = &views[&run.ids.p0];
    let view_late = &views[&run.ids.p2];
    let policy = AlignPolicy::default();

    let span_early = || view_early.span(run.ids.p0, run.ids.pt0);
    let span_late = || view_late.span(run.ids.p2, run.ids.pt2);

    // Latency Unknown: strict → Incomparable; bounded → Conditional.
    let unknown_latency = ObservationSpan {
        latency: LatencyKnowledge::Unknown,
        ..span_late()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &span_early(),
            &unknown_latency,
            &policy,
        )),
        IncomparableReason::LatencyUnknown
    );
    let bounded = AlignPolicy {
        require_known_latency: false,
        unknown_latency_bound_frames: Some(8),
        max_error_frames: 8,
        ..AlignPolicy::default()
    };
    let (_, conditions, _) = expect_conditional(&align_observation_points(
        &span_early(),
        &unknown_latency,
        &bounded,
    ));
    assert!(
        conditions.contains(&AlignmentCondition::LatencyBounded { bound_frames: 8 }),
        "expected LatencyBounded, got {conditions:?}"
    );

    // Channel-count mismatch → FormatMismatch.
    let bad_channels = ObservationSpan {
        format: ObservationFormat {
            channels: 1,
            ..span_late().format
        },
        ..span_late()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &span_early(),
            &bad_channels,
            &policy,
        )),
        IncomparableReason::FormatMismatch
    );

    // Sample-rate mismatch → RateMismatch with both rates reported.
    let bad_rate = ObservationSpan {
        format: ObservationFormat {
            sample_rate: 44_100,
            ..span_late().format
        },
        ..span_late()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(&span_early(), &bad_rate, &policy,)),
        IncomparableReason::RateMismatch {
            a: 48_000,
            b: 44_100
        }
    );
}

/// Identity mismatches on real record-derived spans: mixed precision
/// is `Conditional`, epoch/domain changes are explicit
/// `Incomparable` reasons.
#[test]
fn real_record_spans_reject_identity_mismatches() {
    let run = monitor_run();
    let lost = run.rig.collector.counts().lost_records;
    let views = build_views(&run.evs, lost);
    let view_early = &views[&run.ids.p0];
    let view_late = &views[&run.ids.p2];
    let policy = AlignPolicy::default();

    let span_early = || view_early.span(run.ids.p0, run.ids.pt0);
    let span_late = || view_late.span(run.ids.p2, run.ids.pt2);

    // f32 RT vs f64 native → Conditional{MixedPrecision}.
    let native = ObservationSpan {
        precision: ProbePrecision::F64Native,
        ..span_late()
    };
    let (_, conditions, _) =
        expect_conditional(&align_observation_points(&span_early(), &native, &policy));
    assert!(conditions.contains(&AlignmentCondition::MixedPrecision));

    // Epoch/domain identity changes → explicit mismatch reasons.
    let other_epoch = ObservationSpan {
        clock_mark: ProbeClockMark {
            epoch: ClockEpochId::new(9),
            ..span_late().clock_mark
        },
        ..span_late()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &span_early(),
            &other_epoch,
            &policy,
        )),
        IncomparableReason::EpochMismatch
    );
    let other_domain = ObservationSpan {
        clock_mark: ProbeClockMark {
            domain: ClockDomainId::new(77),
            ..span_late().clock_mark
        },
        ..span_late()
    };
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &span_early(),
            &other_domain,
            &policy,
        )),
        IncomparableReason::DomainMismatch
    );
}

/// Real telemetry overflow (`lost_records > 0`) downgrades every
/// reconstructed interval to `Unknown` — no window may claim validity
/// it cannot prove, and alignment refuses with `CoverageEmpty`.
#[test]
fn telemetry_loss_marks_coverage_unknown() {
    let chain = delay_chain();
    let input = fixture_input();
    // Block 0 emits exactly 6 telemetry records (3 Installed
    // statuses during `drain_commands` + 3 Metric records) — they
    // all fit; blocks 1–2's 6 metrics overflow the full channel.
    let probe_config = ProbeConfig {
        telemetry_capacity: 6,
        ..ProbeConfig::default()
    };
    let (mut rig, ids) = start_monitor(&chain, &probe_config, false);
    run_blocks(&mut rig, &chain, &input, 0..3);
    let mut evs: BTreeMap<ProbeId, Vec<OwnedEv>> = BTreeMap::new();
    drain_into(&mut rig, &mut evs);

    let counts = rig.collector.counts();
    assert!(counts.lost_records > 0, "telemetry overflow expected");
    assert_eq!(counts.lost_blocks, 0);
    let lost = counts.lost_records;
    let views = build_views(&evs, lost);

    for view in views.values() {
        assert_ne!(view.coverage.intervals.len(), 0);
        for iv in &view.coverage.intervals {
            assert_eq!(
                iv.class,
                CoverageClass::Unknown,
                "telemetry loss must downgrade every interval"
            );
        }
    }
    let policy = AlignPolicy::default();
    assert_eq!(
        expect_incomparable(&align_observation_points(
            &views[&ids.p0].span(ids.p0, ids.pt0),
            &views[&ids.p2].span(ids.p2, ids.pt2),
            &policy,
        )),
        IncomparableReason::CoverageEmpty
    );
}
