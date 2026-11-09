//! Issue #309 PROBE-TIME tests — `ProbeClockMark` wiring through
//! `SetClockMark`, `ProbeTimeContext`, the `align_observation_points`
//! three-valued contract, and `ObservationCoverage` reconstruction.
mod common;

use assert_no_alloc::assert_no_alloc;
use common::*;

use ruviform_audio::typed::probe::{
    AlignPolicy, Alignment, AlignmentBasis, AlignmentCondition, CoverageClass, CoverageEvent,
    CoverageInterval, FrameInterval, GenerationRange, IncomparableReason, LatencyKnowledge,
    ObservationCoverage, ObservationFormat, ObservationSpan, ProbeClockMark, ProbeConfig,
    ProbeEvent, ProbeId, ProbeMode, ProbePoint, ProbePrecision, ProbeTimeContext, RecordMeta,
    StatusKind, StatusRecord, UnobservedReason, align_observation_points, pair,
};
use ruviform_audio::typed::{Compiler, GraphProcessor, NodeIo};
use ruviform_audio_core::ChannelLayout;
use ruviform_graph::{Direction, GraphBuilder, GraphSpec, NodeId, NodeKind, PortId, PortRole};
use ruviform_media_core::{
    ClockAnchor, ClockDomainId, ClockEpochId, EngineFrame, TimeBase, TimestampProvenance,
};
use ruviform_rt::RtContext;

const FRAMES: usize = 64;

// ---- fixtures (same shape as typed_probe.rs) ----

struct ProbeChain {
    spec: GraphSpec,
    src_out: PortId,
    proc_in: PortId,
    proc_out: PortId,
    sink_in: PortId,
    proc: NodeId,
}

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
        src_out: sp[0],
        proc_in: pp[0],
        proc_out: pp[1],
        sink_in: kp[0],
        proc,
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
    grt: ruviform_audio::typed::GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    broker: ruviform_audio::typed::probe::ProbeBroker,
    collector: ruviform_audio::typed::probe::ProbeCollector,
}

fn rig(spec: &GraphSpec, probe_config: &ProbeConfig, factory: &mut MockFactory) -> Rig {
    let cfg = config(FRAMES);
    let compiler = Compiler::new(cfg.clone());
    let (mut grt, mut rt) = ruviform_audio::typed::GraphRuntime::new(&cfg, 8, 8);
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
        broker: parts.broker,
        collector: parts.collector,
    }
}

#[derive(Debug)]
enum Ev {
    Metric(ruviform_audio::typed::probe::MetricRecord),
    Status(StatusRecord),
    Cap(RecordMeta),
}

fn collect(rig: &mut Rig) -> Vec<Ev> {
    let mut out = Vec::new();
    rig.collector.drain(|e| match e {
        ProbeEvent::Metric(m) => out.push(Ev::Metric(m)),
        ProbeEvent::Status(s) => out.push(Ev::Status(s)),
        ProbeEvent::Captured(c) => out.push(Ev::Cap(c.meta.expect("captured chunk has meta"))),
        _ => {}
    });
    out
}

#[expect(clippy::cast_precision_loss)]
fn feed_ramp(rt: &mut ruviform_audio::typed::RtRuntime, port: PortId, offset: f32) {
    let l: Vec<f32> = (0..FRAMES)
        .map(|i| (i as f32 + offset) / FRAMES as f32)
        .collect();
    let r: Vec<f32> = (0..FRAMES)
        .map(|i| ((i as f32) * 0.5 + offset) / FRAMES as f32)
        .collect();
    feed(rt, port, &[&l, &r]);
}

fn ctx() -> RtContext {
    RtContext::mint()
}

// ---- synthetic span helpers ----

/// Real ids harvested from one installed rig — `ProbeId`/`NodeId`/
/// `PortId` cannot be fabricated outside their crates.
struct SpanIds {
    a: ProbeId,
    b: ProbeId,
    pa: ProbePoint,
    pb: ProbePoint,
    node: NodeId,
    port_a: PortId,
}

fn span_ids() -> SpanIds {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let pa = ProbePoint {
        node: c.proc,
        port: c.proc_in,
    };
    let pb = ProbePoint {
        node: c.proc,
        port: c.proc_out,
    };
    let a = rig
        .broker
        .install(&rig.grt, &c.spec, pa, ProbeMode::Monitor)
        .unwrap();
    let b = rig
        .broker
        .install(&rig.grt, &c.spec, pb, ProbeMode::Monitor)
        .unwrap();
    SpanIds {
        a,
        b,
        pa,
        pb,
        node: c.proc,
        port_a: c.proc_in,
    }
}

fn mark(domain: u64, epoch: u64) -> ProbeClockMark {
    ProbeClockMark {
        domain: ClockDomainId::new(domain),
        epoch: ClockEpochId::new(epoch),
    }
}

fn meta(
    probe: ProbeId,
    ids: &SpanIds,
    port: PortId,
    pos: u64,
    frames: u32,
    clock: ProbeClockMark,
) -> RecordMeta {
    RecordMeta {
        probe,
        node: ids.node,
        port,
        generation: 1,
        sample_position: pos,
        frames,
        channels: 2,
        sample_rate: 48_000,
        effective_latency: 0,
        clock,
    }
}

fn status(
    probe: ProbeId,
    ids: &SpanIds,
    port: PortId,
    pos: u64,
    clock: ProbeClockMark,
    kind: StatusKind,
) -> StatusRecord {
    StatusRecord {
        probe,
        node: ids.node,
        port,
        generation: 1,
        sample_position: pos,
        clock,
        kind,
    }
}

fn fmt() -> ObservationFormat {
    ObservationFormat {
        channels: 2,
        sample_rate: 48_000,
        channel_layout: ChannelLayout::Stereo,
    }
}

fn valid_coverage(start: u64, end: u64, clock: ProbeClockMark) -> ObservationCoverage {
    ObservationCoverage {
        intervals: vec![CoverageInterval {
            start,
            end,
            class: CoverageClass::Valid,
            mark: clock,
        }],
    }
}

fn span<'a>(
    probe: ProbeId,
    point: ProbePoint,
    clock_mark: ProbeClockMark,
    interval: FrameInterval,
    coverage: &'a ObservationCoverage,
    latency: LatencyKnowledge,
    context: Option<&'a ProbeTimeContext>,
) -> ObservationSpan<'a> {
    ObservationSpan {
        probe,
        point,
        clock_mark,
        interval,
        generation_range: GenerationRange::single(1),
        format: fmt(),
        latency,
        coverage,
        context,
        precision: ProbePrecision::F32Rt,
    }
}

/// Shared-cursor spans over `[0, 64)` with full valid coverage and
/// known-but-different latencies.
fn cursor_pair<'a>(
    ids: &SpanIds,
    clock: ProbeClockMark,
    cov_a: &'a ObservationCoverage,
    cov_b: &'a ObservationCoverage,
    lat_a: u64,
    lat_b: u64,
) -> (ObservationSpan<'a>, ObservationSpan<'a>) {
    (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 64),
            cov_a,
            LatencyKnowledge::Known { frames: lat_a },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(0, 64),
            cov_b,
            LatencyKnowledge::Known { frames: lat_b },
            None,
        ),
    )
}

// ---- clock mark wiring (integration) ----

/// Unwired decks stamp `UNKNOWN`; `wire_clock_mark` delivers the
/// minted `(domain, epoch)`; `bump_epoch` propagates the new epoch.
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "fixture ids mirror typed_probe.rs naming"
)]
fn clock_mark_wiring_stamps_records() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let p = rig
        .broker
        .install(
            &rig.grt,
            &c.spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();

    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    let m = evs
        .iter()
        .find_map(|e| match e {
            Ev::Metric(m) if m.meta.probe == p => Some(m),
            _ => None,
        })
        .expect("metric record");
    assert!(m.meta.clock.is_unknown(), "unwired deck must stamp UNKNOWN");

    rig.broker.wire_clock_mark().unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    let m = evs
        .iter()
        .find_map(|e| match e {
            Ev::Metric(m) if m.meta.probe == p => Some(m),
            _ => None,
        })
        .expect("metric record");
    assert_eq!(m.meta.clock, rig.broker.time_context().mark());
    assert_eq!(m.meta.clock.domain, rig.broker.time_context().domain);
    assert_eq!(m.meta.clock.epoch.get(), 0);

    let e = rig.broker.bump_epoch().unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    let m = evs
        .iter()
        .find_map(|e| match e {
            Ev::Metric(m) if m.meta.probe == p => Some(m),
            _ => None,
        })
        .expect("metric record");
    assert_eq!(m.meta.clock.epoch, e);
    assert_eq!(e.get(), 1);
}

/// `SetClockMark` is a `Copy` store on the RT side — queueing one
/// inside the alloc-check window allocates nothing.
#[test]
fn set_clock_mark_is_alloc_free_on_rt_path() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    rig.broker
        .install(
            &rig.grt,
            &c.spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    // Apply the bind outside the window.
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    rig.collector.drain(|_| {});
    // Queue the mark command, then process inside the window — the
    // drain applies `SetClockMark` with a plain assignment.
    rig.broker.wire_clock_mark().unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    let rc = ctx();
    assert_no_alloc(|| rig.rt.process(FRAMES, &rc).unwrap());
    // The mark applied inside the window is visible on the next
    // block's records.
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &rc).unwrap();
    let evs = collect(&mut rig);
    let wired = evs.iter().any(|e| match e {
        Ev::Metric(m) => m.meta.clock == rig.broker.time_context().mark(),
        Ev::Status(s) => s.clock == rig.broker.time_context().mark(),
        Ev::Cap(meta) => meta.clock == rig.broker.time_context().mark(),
    });
    assert!(wired, "SetClockMark applied inside RT window");
}

/// The clock mark is record metadata only — sink PCM is bit-identical
/// whether or not the series was wired (Issue #308 guarantee kept).
#[test]
fn clock_mark_does_not_change_output_pcm() {
    let build = |wire: bool| {
        let c = probe_chain();
        let mut f = MockFactory::default();
        f.pending.insert(c.proc, gain2());
        let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
        rig.broker
            .install(
                &rig.grt,
                &c.spec,
                ProbePoint {
                    node: c.proc,
                    port: c.proc_out,
                },
                ProbeMode::Monitor,
            )
            .unwrap();
        if wire {
            rig.broker.wire_clock_mark().unwrap();
            rig.broker.bump_epoch().unwrap();
        }
        // Apply any pending commands, then observe two blocks.
        feed_ramp(&mut rig.rt, c.src_out, 0.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
        feed_ramp(&mut rig.rt, c.src_out, 1.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
        let pcm = drain(&mut rig.rt, c.sink_in);
        (c, rig, pcm)
    };
    let (c_a, mut rig_a, pcm_a) = build(true);
    let (_c_b, _rig_b, pcm_b) = build(false);
    let _ = c_a;
    assert_eq!(pcm_a, pcm_b, "clock mark must not alter sink PCM");
    let _ = &mut rig_a;
}

// ---- align_observation_points: fail-closed identity ----

/// Rule: unknown clock identity never aligns — not even UNKNOWN vs
/// UNKNOWN, and not UNKNOWN vs a real mark.
#[test]
fn unknown_clock_never_aligns() {
    let ids = span_ids();
    let cov = valid_coverage(0, 64, ProbeClockMark::UNKNOWN);
    let policy = AlignPolicy::default();

    let (a, b) = cursor_pair(&ids, ProbeClockMark::UNKNOWN, &cov, &cov, 0, 0);
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::UnknownClock,
        } => {}
        other => panic!("expected UnknownClock, got {other:?}"),
    }

    // One unknown side is equally fatal.
    let known_cov = valid_coverage(0, 64, mark(100, 0));
    let (a, b) = cursor_pair(&ids, mark(100, 0), &known_cov, &cov, 0, 0);
    let a = ObservationSpan {
        clock_mark: ProbeClockMark::UNKNOWN,
        ..a
    };
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::UnknownClock,
        } => {}
        other => panic!("expected UnknownClock, got {other:?}"),
    }
}

/// Rule: domain/epoch mismatch without anchor evidence is
/// incomparable — numeric proximity of `sample_position` is not
/// alignment evidence.
#[test]
fn domain_and_epoch_mismatch_are_incomparable() {
    let ids = span_ids();
    let policy = AlignPolicy::default();
    let cov_a = valid_coverage(0, 64, mark(100, 0));
    let cov_b = valid_coverage(0, 64, mark(200, 0));
    let (a, b) = cursor_pair(&ids, mark(100, 0), &cov_a, &cov_b, 0, 0);
    let b = ObservationSpan {
        clock_mark: mark(200, 0),
        ..b
    };
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::DomainMismatch,
        } => {}
        other => panic!("expected DomainMismatch, got {other:?}"),
    }

    let cov_b = valid_coverage(0, 64, mark(100, 1));
    let (a, b) = cursor_pair(&ids, mark(100, 0), &cov_a, &cov_b, 0, 0);
    let b = ObservationSpan {
        clock_mark: mark(100, 1),
        ..b
    };
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::EpochMismatch,
        } => {}
        other => panic!("expected EpochMismatch, got {other:?}"),
    }
}

// ---- align_observation_points: evidence-bearing results ----

/// Same `(domain, epoch)`, known latencies, full coverage →
/// `Aligned{SharedCursor}` whose offset equals the latency difference
/// in B-side frame units.
#[test]
fn shared_cursor_aligns_with_latency_offset() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let policy = AlignPolicy::default();
    let (a, b) = cursor_pair(&ids, clock, &cov, &cov, 10, 20);
    match align_observation_points(&a, &b, &policy) {
        Alignment::Aligned {
            basis: AlignmentBasis::SharedCursor { domain, epoch },
            offset,
            error_budget,
            overlap,
        } => {
            assert_eq!(domain, clock.domain);
            assert_eq!(epoch, clock.epoch);
            assert_eq!(offset, 10, "offset = lat_b - lat_a");
            assert_eq!(error_budget, 0);
            // A's window shifts +10 into B coordinates → [10,74);
            // the joint overlap with B's [0,64) is [10,64).
            assert_eq!(
                overlap,
                Some(FrameInterval::new(10, 64)),
                "latency-shifted joint coverage"
            );
        }
        other => panic!("expected Aligned, got {other:?}"),
    }
}

/// Differing declared rates without conversion permission fail;
/// with it, the rational conversion runs and the residue lands in
/// `error_budget` — never silently zero.
#[test]
fn rate_mismatch_requires_explicit_conversion() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let policy = AlignPolicy::default();
    let (a, mut b) = cursor_pair(&ids, clock, &cov, &cov, 0, 0);
    b.format.sample_rate = 44_100;

    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::RateMismatch { a: ra, b: rb },
        } => {
            assert_eq!((ra, rb), (48_000, 44_100));
        }
        other => panic!("expected RateMismatch, got {other:?}"),
    }

    let permissive = AlignPolicy {
        allow_rate_conversion: true,
        max_error_frames: 16,
        ..AlignPolicy::default()
    };
    match align_observation_points(&a, &b, &permissive) {
        Alignment::Aligned { error_budget, .. } | Alignment::Conditional { error_budget, .. } => {
            assert!(error_budget > 0, "conversion residue must be budgeted");
        }
        other => panic!("expected aligned/conditional with budget, got {other:?}"),
    }
}

/// `LatencyKnowledge::Unknown` is never a known zero: strict policy
/// refuses, bounded policy yields `Conditional{LatencyBounded}`.
#[test]
fn unknown_latency_is_not_treated_as_zero() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let (a, b) = cursor_pair(&ids, clock, &cov, &cov, 0, 0);
    let a = ObservationSpan {
        latency: LatencyKnowledge::Unknown,
        ..a
    };
    let policy = AlignPolicy::default();
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::LatencyUnknown,
        } => {}
        other => panic!("expected LatencyUnknown, got {other:?}"),
    }

    let bounded = AlignPolicy {
        require_known_latency: false,
        unknown_latency_bound_frames: Some(8),
        max_error_frames: 16,
        ..AlignPolicy::default()
    };
    match align_observation_points(&a, &b, &bounded) {
        Alignment::Conditional { conditions, .. } => {
            assert!(
                conditions.contains(&AlignmentCondition::LatencyBounded { bound_frames: 8 }),
                "expected LatencyBounded condition, got {conditions:?}"
            );
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
}

/// Generation ranges that differ or cross a rebind are observation
/// discontinuities — `require_same_generation` refuses, permissive
/// policy marks `CrossGeneration`.
#[test]
fn generation_span_is_not_coordinate_continuity() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let (mut a, b) = cursor_pair(&ids, clock, &cov, &cov, 0, 0);
    a.generation_range = GenerationRange { first: 1, last: 2 };
    let policy = AlignPolicy::default();
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::GenerationSpan,
        } => {}
        other => panic!("expected GenerationSpan, got {other:?}"),
    }

    let permissive = AlignPolicy {
        require_same_generation: false,
        ..AlignPolicy::default()
    };
    match align_observation_points(&a, &b, &permissive) {
        Alignment::Conditional { conditions, .. } => {
            assert!(conditions.contains(&AlignmentCondition::CrossGeneration));
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
}

/// Channel/layout mismatch is a different signal shape — never
/// aligned regardless of clock identity.
#[test]
fn format_mismatch_is_incomparable() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let (a, mut b) = cursor_pair(&ids, clock, &cov, &cov, 0, 0);
    b.format.channels = 1;
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Incomparable {
            reason: IncomparableReason::FormatMismatch,
        } => {}
        other => panic!("expected FormatMismatch, got {other:?}"),
    }
}

/// Coverage gaps inside the compared window demote to `Conditional`
/// with explicit excluded intervals — or `Incomparable` when the
/// policy forbids conditional results. The gap is never reported as
/// valid (and never as "silence").
#[test]
fn partial_coverage_is_conditional_with_exclusions() {
    let ids = span_ids();
    let clock = mark(100, 0);
    // A observed [0,32) and [64,96); gap [32,64) unobserved.
    let cov_a = ObservationCoverage {
        intervals: vec![
            CoverageInterval {
                start: 0,
                end: 32,
                class: CoverageClass::Valid,
                mark: clock,
            },
            CoverageInterval {
                start: 64,
                end: 96,
                class: CoverageClass::Valid,
                mark: clock,
            },
        ],
    };
    let cov_b = valid_coverage(0, 96, clock);
    let policy = AlignPolicy::default();
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 96),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(0, 96),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
    );
    match align_observation_points(&a, &b, &policy) {
        Alignment::Conditional {
            conditions,
            excluded_intervals,
            ..
        } => {
            assert!(conditions.contains(&AlignmentCondition::PartialCoverage));
            assert_eq!(excluded_intervals, vec![FrameInterval::new(32, 64)]);
        }
        other => panic!("expected Conditional, got {other:?}"),
    }

    let strict = AlignPolicy {
        allow_conditional: false,
        ..AlignPolicy::default()
    };
    match align_observation_points(&a, &b, &strict) {
        Alignment::Incomparable { .. } => {}
        other => panic!("expected Incomparable, got {other:?}"),
    }
}

/// No jointly valid coverage — disjoint windows — is
/// `CoverageEmpty`, not a vacuous `Aligned`.
#[test]
fn empty_joint_coverage_is_incomparable() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov_a = valid_coverage(0, 32, clock);
    let cov_b = valid_coverage(64, 96, clock);
    let policy = AlignPolicy::default();
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 32),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(64, 96),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
    );
    match align_observation_points(&a, &b, &policy) {
        Alignment::Incomparable {
            reason: IncomparableReason::CoverageEmpty,
        } => {}
        other => panic!("expected CoverageEmpty, got {other:?}"),
    }
}

/// A generation boundary or rebind drop inside the compared window
/// is a hard discontinuity — incomparable regardless of coverage.
#[test]
fn discontinuity_inside_window_is_incomparable() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov_a = ObservationCoverage {
        intervals: vec![
            CoverageInterval {
                start: 0,
                end: 32,
                class: CoverageClass::Valid,
                mark: clock,
            },
            CoverageInterval {
                start: 32,
                end: 64,
                class: CoverageClass::GenerationBoundary,
                mark: clock,
            },
        ],
    };
    let cov_b = valid_coverage(0, 64, clock);
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 64),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(0, 64),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
    );
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Incomparable {
            reason: IncomparableReason::DiscontinuityInWindow,
        } => {}
        other => panic!("expected DiscontinuityInWindow, got {other:?}"),
    }
}

/// A coverage interval carrying a *different* clock mark inside the
/// span's window is a series discontinuity — the whole window is
/// incomparable rather than partially trusted.
#[test]
fn mixed_clock_marks_in_window_are_incomparable() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let other = mark(100, 1);
    let cov_a = ObservationCoverage {
        intervals: vec![
            CoverageInterval {
                start: 0,
                end: 32,
                class: CoverageClass::Valid,
                mark: clock,
            },
            CoverageInterval {
                start: 32,
                end: 64,
                class: CoverageClass::Valid,
                mark: other,
            },
        ],
    };
    let cov_b = valid_coverage(0, 64, clock);
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 64),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(0, 64),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
    );
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Incomparable {
            reason: IncomparableReason::DiscontinuityInWindow,
        } => {}
        other => panic!("expected DiscontinuityInWindow, got {other:?}"),
    }
}

// ---- anchor-based EngineFrame basis ----

fn ctx_with_anchor(
    domain: ClockDomainId,
    engine_epoch: u64,
    engine_frame: u64,
    provenance: TimestampProvenance,
) -> ProbeTimeContext {
    let rate = 48_000;
    ProbeTimeContext {
        domain,
        epoch: ClockEpochId::new(0),
        runtime_instance: 1,
        sample_rate: rate,
        provenance,
        anchors: vec![ClockAnchor {
            domain,
            stamp_ticks: 0,
            stamp_base: TimeBase::sample_rate(rate).unwrap(),
            frame: EngineFrame::new(ClockEpochId::new(engine_epoch), engine_frame),
            calibration: TimestampProvenance::Carried,
        }],
        generation_window: None,
    }
}

/// Different `(domain, epoch)` marks align only through explicit
/// anchor evidence into one engine epoch — `EngineFrame` basis.
#[test]
fn anchor_evidence_enables_engine_frame_basis() {
    let ids = span_ids();
    let d_a = ClockDomainId::new(100);
    let d_b = ClockDomainId::new(200);
    let ctx_a = ctx_with_anchor(d_a, 7, 1_000, TimestampProvenance::Derived);
    let ctx_b = ctx_with_anchor(d_b, 7, 1_000, TimestampProvenance::Derived);
    let cov_a = valid_coverage(0, 64, mark(100, 0));
    let cov_b = valid_coverage(0, 64, mark(200, 0));
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            mark(100, 0),
            FrameInterval::new(0, 64),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            Some(&ctx_a),
        ),
        span(
            ids.b,
            ids.pb,
            mark(200, 0),
            FrameInterval::new(0, 64),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            Some(&ctx_b),
        ),
    );
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Aligned {
            basis: AlignmentBasis::EngineFrame { epoch },
            ..
        } => assert_eq!(epoch.get(), 7),
        other => panic!("expected Aligned EngineFrame, got {other:?}"),
    }
}

/// Anchors placing the spans in different engine epochs are
/// different frame series — `SeriesMismatch`, never a numeric
/// comparison.
#[test]
fn divergent_engine_epochs_are_series_mismatch() {
    let ids = span_ids();
    let d_a = ClockDomainId::new(100);
    let d_b = ClockDomainId::new(200);
    let ctx_a = ctx_with_anchor(d_a, 7, 1_000, TimestampProvenance::Derived);
    let ctx_b = ctx_with_anchor(d_b, 8, 1_000, TimestampProvenance::Derived);
    let cov_a = valid_coverage(0, 64, mark(100, 0));
    let cov_b = valid_coverage(0, 64, mark(200, 0));
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            mark(100, 0),
            FrameInterval::new(0, 64),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            Some(&ctx_a),
        ),
        span(
            ids.b,
            ids.pb,
            mark(200, 0),
            FrameInterval::new(0, 64),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            Some(&ctx_b),
        ),
    );
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Incomparable {
            reason: IncomparableReason::SeriesMismatch,
        } => {}
        other => panic!("expected SeriesMismatch, got {other:?}"),
    }
}

/// `Estimated` provenance anywhere in the evidence chain demotes the
/// result to `Conditional` — or `Incomparable` when conditional
/// results are disallowed.
#[test]
fn estimated_provenance_demotes_to_conditional() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let cov = valid_coverage(0, 64, clock);
    let ctx_a = ProbeTimeContext {
        domain: clock.domain,
        epoch: clock.epoch,
        runtime_instance: 1,
        sample_rate: 48_000,
        provenance: TimestampProvenance::Estimated,
        anchors: Vec::new(),
        generation_window: None,
    };
    let ctx_b = ProbeTimeContext {
        provenance: TimestampProvenance::Derived,
        ..ctx_a.clone()
    };
    let (a, b) = cursor_pair(&ids, clock, &cov, &cov, 0, 0);
    let a = ObservationSpan {
        context: Some(&ctx_a),
        ..a
    };
    let b = ObservationSpan {
        context: Some(&ctx_b),
        ..b
    };
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Conditional { conditions, .. } => {
            assert!(conditions.contains(&AlignmentCondition::ProvenanceEstimated));
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
    let strict = AlignPolicy {
        allow_conditional: false,
        ..AlignPolicy::default()
    };
    match align_observation_points(&a, &b, &strict) {
        Alignment::Incomparable {
            reason: IncomparableReason::EstimatedProvenance,
        } => {}
        other => panic!("expected EstimatedProvenance, got {other:?}"),
    }
}

// ---- ObservationCoverage reconstruction ----

/// Consecutive observed blocks reconstruct as one `Valid` interval —
/// measured silence is `Valid` too (a zero-valued metric is still an
/// observation), never collapsed into "missing".
#[test]
fn coverage_reconstructs_valid_and_measured_silence() {
    let ids = span_ids();
    let clock = mark(100, 0);
    // Two back-to-back blocks, second fed digital silence — both are
    // observations.
    let evs = vec![
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 0, 64, clock)),
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 64, 64, clock)),
    ];
    let cov = ObservationCoverage::reconstruct(ids.a, 0, evs);
    assert_eq!(
        cov.intervals,
        vec![CoverageInterval {
            start: 0,
            end: 128,
            class: CoverageClass::Valid,
            mark: clock,
        }],
        "contiguous observed blocks merge into one Valid interval"
    );
    // Unexplained gaps between observed blocks downgrade to Unknown,
    // not Valid and not "silence".
    let evs = vec![
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 0, 64, clock)),
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 128, 64, clock)),
    ];
    let cov = ObservationCoverage::reconstruct(ids.a, 0, evs);
    assert_eq!(cov.intervals.len(), 3);
    assert_eq!(cov.intervals[1].class, CoverageClass::Unknown);
    assert_eq!((cov.intervals[1].start, cov.intervals[1].end), (64, 128));
}

/// Status records classify their span: `Unobserved`/`LostBlocks`/
/// `Off` are distinct from `Valid` — loss masks never borrow the
/// "measured" label.
#[test]
fn coverage_reconstruction_classifies_status_kinds() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let evs = vec![
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 0, 64, clock)),
        CoverageEvent::Status(status(
            ids.a,
            &ids,
            ids.port_a,
            64,
            clock,
            StatusKind::Unobserved {
                reason: UnobservedReason::Stale,
            },
        )),
        CoverageEvent::Status(status(
            ids.a,
            &ids,
            ids.port_a,
            128,
            clock,
            StatusKind::LostBlocks,
        )),
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 192, 64, clock)),
    ];
    let cov = ObservationCoverage::reconstruct(ids.a, 0, evs);
    let at = |s: u64| {
        cov.intervals.iter().find(|i| i.start == s).map_or_else(
            || panic!("no interval at {s}: {:?}", cov.intervals),
            |i| i.class,
        )
    };
    assert_eq!(at(0), CoverageClass::Valid);
    assert_eq!(
        at(64),
        CoverageClass::Unobserved {
            reason: UnobservedReason::Stale
        }
    );
    assert_eq!(at(128), CoverageClass::LostBlocks);
    assert_eq!(at(192), CoverageClass::Valid);
}

/// `ModeChanged{Off}` opens a persistent `Off` state closed by the
/// resume transition — probe-off is "not observed by design", kept
/// distinct from loss.
#[test]
fn coverage_reconstruction_tracks_off_state() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let evs = vec![
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 0, 64, clock)),
        CoverageEvent::Status(status(
            ids.a,
            &ids,
            ids.port_a,
            64,
            clock,
            StatusKind::ModeChanged { to: ProbeMode::Off },
        )),
        CoverageEvent::Status(status(
            ids.a,
            &ids,
            ids.port_a,
            128,
            clock,
            StatusKind::ModeChanged {
                to: ProbeMode::Monitor,
            },
        )),
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 128, 64, clock)),
    ];
    let cov = ObservationCoverage::reconstruct(ids.a, 0, evs);
    let off = cov
        .intervals
        .iter()
        .find(|i| i.class == CoverageClass::Off)
        .expect("Off interval");
    assert_eq!((off.start, off.end), (64, 128));
}

/// A positive `lost_records` delta downgrades every interval to
/// `Unknown` — the counter carries no position, so no window may
/// claim validity it cannot prove.
#[test]
fn telemetry_loss_downgrades_coverage_to_unknown() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let evs = vec![CoverageEvent::Data(meta(
        ids.a, &ids, ids.port_a, 0, 64, clock,
    ))];
    let cov = ObservationCoverage::reconstruct(ids.a, 1, evs);
    assert!(
        cov.intervals
            .iter()
            .all(|i| i.class == CoverageClass::Unknown),
        "lost_records>0 must downgrade all intervals: {:?}",
        cov.intervals
    );
}

/// Reconstructed coverage feeds `align_observation_points`: a span
/// whose coverage contains an `Unobserved` gap yields
/// `Conditional{PartialCoverage}` — the gap is named, never hidden.
#[test]
fn reconstructed_coverage_drives_conditional_alignment() {
    let ids = span_ids();
    let clock = mark(100, 0);
    let evs = vec![
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 0, 32, clock)),
        CoverageEvent::Status(status(
            ids.a,
            &ids,
            ids.port_a,
            32,
            clock,
            StatusKind::Unobserved {
                reason: UnobservedReason::BorrowConflict,
            },
        )),
        CoverageEvent::Data(meta(ids.a, &ids, ids.port_a, 96, 32, clock)),
    ];
    let cov_a = ObservationCoverage::reconstruct(ids.a, 0, evs);
    // The unobserved status covers [32,96) — next event pos is 96.
    assert!(cov_a.intervals.iter().any(|i| i.class
        == CoverageClass::Unobserved {
            reason: UnobservedReason::BorrowConflict
        }));

    let cov_b = valid_coverage(0, 128, clock);
    let (a, b) = (
        span(
            ids.a,
            ids.pa,
            clock,
            FrameInterval::new(0, 128),
            &cov_a,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
        span(
            ids.b,
            ids.pb,
            clock,
            FrameInterval::new(0, 128),
            &cov_b,
            LatencyKnowledge::Known { frames: 0 },
            None,
        ),
    );
    match align_observation_points(&a, &b, &AlignPolicy::default()) {
        Alignment::Conditional { conditions, .. } => {
            assert!(conditions.contains(&AlignmentCondition::PartialCoverage));
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
}

// ---- integration: real records → spans → alignment ----

/// Two probes on the same wired series share `(domain, epoch)` —
/// their drained records align `SharedCursor` with the latency
/// difference as offset.
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "fixture ids mirror typed_probe.rs naming"
)]
fn same_series_probes_align_with_latency_offset() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let pa = ProbePoint {
        node: c.proc,
        port: c.proc_in,
    };
    let pb = ProbePoint {
        node: c.proc,
        port: c.proc_out,
    };
    let a = rig
        .broker
        .install(&rig.grt, &c.spec, pa, ProbeMode::Monitor)
        .unwrap();
    let b = rig
        .broker
        .install(&rig.grt, &c.spec, pb, ProbeMode::Monitor)
        .unwrap();
    rig.broker.wire_clock_mark().unwrap();

    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 1.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);

    let mut metas: Vec<RecordMeta> = evs
        .iter()
        .filter_map(|e| match e {
            Ev::Metric(m) => Some(m.meta),
            Ev::Cap(meta) => Some(*meta),
            Ev::Status(_) => None,
        })
        .collect();
    metas.sort_by_key(|m| (m.probe.get(), m.sample_position));
    let meta_a = metas.iter().find(|m| m.probe == a).unwrap();
    let meta_b = metas.iter().find(|m| m.probe == b).unwrap();
    assert_eq!(meta_a.clock, meta_b.clock, "same series, same mark");
    assert!(!meta_a.clock.is_unknown());

    // Reconstruct coverage per probe over the observed window.
    let cov_a = ObservationCoverage::reconstruct(
        a,
        0,
        evs.iter().map(|e| match e {
            Ev::Metric(m) => CoverageEvent::Data(m.meta),
            Ev::Status(s) => CoverageEvent::Status(*s),
            Ev::Cap(meta) => CoverageEvent::Data(*meta),
        }),
    );
    let cov_b = ObservationCoverage::reconstruct(
        b,
        0,
        evs.iter().map(|e| match e {
            Ev::Metric(m) => CoverageEvent::Data(m.meta),
            Ev::Status(s) => CoverageEvent::Status(*s),
            Ev::Cap(meta) => CoverageEvent::Data(*meta),
        }),
    );

    let ctx_ref = rig.broker.time_context();
    let span_a = ObservationSpan {
        probe: a,
        point: pa,
        clock_mark: meta_a.clock,
        interval: FrameInterval::new(0, 2 * FRAMES as u64),
        generation_range: GenerationRange::single(meta_a.generation),
        format: fmt(),
        latency: LatencyKnowledge::Known {
            frames: u64::from(meta_a.effective_latency),
        },
        coverage: &cov_a,
        context: Some(ctx_ref),
        precision: ProbePrecision::F32Rt,
    };
    let span_b = ObservationSpan {
        probe: b,
        point: pb,
        clock_mark: meta_b.clock,
        interval: FrameInterval::new(0, 2 * FRAMES as u64),
        generation_range: GenerationRange::single(meta_b.generation),
        format: fmt(),
        latency: LatencyKnowledge::Known {
            frames: u64::from(meta_b.effective_latency),
        },
        coverage: &cov_b,
        context: Some(ctx_ref),
        precision: ProbePrecision::F32Rt,
    };
    match align_observation_points(&span_a, &span_b, &AlignPolicy::default()) {
        Alignment::Aligned {
            basis: AlignmentBasis::SharedCursor { .. },
            offset,
            ..
        } => {
            let expect = i64::from(meta_b.effective_latency) - i64::from(meta_a.effective_latency);
            assert_eq!(offset, expect, "offset tracks effective-latency delta");
        }
        other => panic!("expected Aligned SharedCursor, got {other:?}"),
    }
}
