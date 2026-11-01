//! Issue #185 generic-probe `PoC` acceptance tests — capability
//! enumeration, Off/Monitor/Capture, same-span Peak/RMS, bounded PCM
//! capture with metadata, loss/fault injection, and RT alloc check.
mod common;

use assert_no_alloc::assert_no_alloc;
use common::*;

use ruviform_audio::typed::probe::{
    CaptureConfig, PointAvailability, ProbeConfig, ProbeError, ProbeEvent, ProbeMode, ProbePoint,
    StatusKind, StopReason, UnobservedReason, capabilities, pair,
};
use ruviform_audio::typed::{Compiler, GraphProcessor, GraphRuntime, NodeIo};
use ruviform_graph::{
    Direction, GraphBuilder, GraphSpec, NodeId, NodeKind, PortFormat, PortId, PortRole,
};
use ruviform_rt::RtContext;

const FRAMES: usize = 64;

// ---- fixtures ----

struct ProbeChain {
    spec: GraphSpec,
    src_out: PortId,
    proc_in: PortId,
    proc_out: PortId,
    sink_in: PortId,
    proc: NodeId,
}

/// Source → gain×2 processor → sink. `ProbePoint` declared on
/// `proc_in`/`proc_out`; `src_out` declared too; `sink_in` undeclared.
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

/// Pure gain×2 processor (stateless — exact metric ratios).
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
    grt: GraphRuntime,
    rt: ruviform_audio::typed::RtRuntime,
    broker: ruviform_audio::typed::probe::ProbeBroker,
    collector: ruviform_audio::typed::probe::ProbeCollector,
}

fn rig(spec: &GraphSpec, probe_config: &ProbeConfig, factory: &mut MockFactory) -> Rig {
    let cfg = config(FRAMES);
    let compiler = Compiler::new(cfg.clone());
    let (mut grt, mut rt) = GraphRuntime::new(&cfg, 8, 8);
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
    Status(ruviform_audio::typed::probe::StatusRecord),
    Cap {
        meta: ruviform_audio::typed::probe::RecordMeta,
        frames: usize,
        channels: usize,
        data: Vec<f32>,
    },
}

impl Ev {
    fn metric(&self) -> Option<&ruviform_audio::typed::probe::MetricRecord> {
        match self {
            Self::Metric(m) => Some(m),
            _ => None,
        }
    }
    fn status(&self) -> Option<&ruviform_audio::typed::probe::StatusRecord> {
        match self {
            Self::Status(s) => Some(s),
            _ => None,
        }
    }
}

fn collect(rig: &mut Rig) -> Vec<Ev> {
    let mut out = Vec::new();
    rig.collector.drain(|e| match e {
        ProbeEvent::Metric(m) => out.push(Ev::Metric(m)),
        ProbeEvent::Status(s) => out.push(Ev::Status(s)),
        ProbeEvent::Captured(c) => out.push(Ev::Cap {
            meta: c.meta.expect("captured chunk has meta"),
            frames: c.frames,
            channels: c.channels,
            data: c.data.to_vec(),
        }),
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

// ---- tests ----

/// ex01: capability report covers every port with the five
/// classifications, deterministic across re-enumeration.
#[test]
fn capability_report_classifies_and_is_deterministic() {
    let mut extra = GraphBuilder::new();
    // Rebuild a spec that also carries a non-audio port + an
    // unconnected audio input (Unavailable).
    let (src, sp) = extra.add_node(&node(
        NodeKind::Source,
        vec![port_probe(Direction::Output, PortRole::Main, stereo())],
    ));
    let (proc, pp) = extra.add_node(&node(
        NodeKind::Processor,
        vec![
            port_probe(Direction::Input, PortRole::Main, stereo()),
            port(Direction::Input, PortRole::Send, stereo()), // unconnected
            port_probe(Direction::Output, PortRole::Main, stereo()),
            port(Direction::Input, PortRole::Main, PortFormat::Control), // non-audio
        ],
    ));
    let (sink, kp) = extra.add_node(&node(
        NodeKind::Sink,
        vec![port(Direction::Input, PortRole::Main, stereo())],
    ));
    extra.connect(&conn(pref(src, sp[0]), pref(proc, pp[0])));
    extra.connect(&conn(pref(proc, pp[2]), pref(sink, kp[0])));
    let spec = extra.build();
    let mut f = MockFactory::default();
    f.pending.insert(proc, gain2());
    let rig = rig(&spec, &ProbeConfig::default(), &mut f);

    let report = capabilities(&spec, rig.grt.published_plan());
    assert_eq!(report.generation, 1);
    let report2 = capabilities(&spec, rig.grt.published_plan());
    assert_eq!(report.points.len(), report2.points.len());
    for (a, b) in report.points.iter().zip(&report2.points) {
        assert_eq!(a.availability, b.availability);
        assert_eq!(a.point.port, b.point.port);
    }
    let class = |port: PortId| {
        report
            .points
            .iter()
            .find(|p| p.point.port == port)
            .map(|p| p.availability)
    };
    assert_eq!(class(sp[0]), Some(PointAvailability::Configured));
    assert_eq!(class(pp[0]), Some(PointAvailability::Configured));
    assert_eq!(class(pp[1]), Some(PointAvailability::Unavailable));
    assert_eq!(class(pp[2]), Some(PointAvailability::Configured));
    assert_eq!(class(pp[3]), Some(PointAvailability::Unsupported));
    assert_eq!(class(kp[0]), Some(PointAvailability::Inferred));
}

/// ex02 + ex03: mode transitions emit status; probes on/off leave
/// the boundary output bit-identical.
#[test]
fn mode_transitions_and_output_invariance() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;

    let p_in = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    let p_out = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();

    // Block 1: probes ON.
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let on = drain(&mut rig.rt, c.sink_in);
    let evs = collect(&mut rig);
    assert!(
        evs.iter()
            .filter_map(Ev::status)
            .any(|s| { matches!(s.kind, StatusKind::Installed) && s.probe == p_in })
    );
    assert_eq!(evs.iter().filter_map(Ev::metric).count(), 2);

    // Turn both Off.
    rig.broker.set_mode(p_in, ProbeMode::Off).unwrap();
    rig.broker.set_mode(p_out, ProbeMode::Off).unwrap();

    // Block 2: probes OFF — output must be bit-identical for the
    // same input, and no records may be emitted.
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let off = drain(&mut rig.rt, c.sink_in);
    assert_eq!(on, off, "probe on/off changed audio output");
    let evs = collect(&mut rig);
    assert!(
        evs.iter()
            .filter_map(Ev::status)
            .any(|s| { matches!(s.kind, StatusKind::ModeChanged { to: ProbeMode::Off }) })
    );
    assert_eq!(evs.iter().filter_map(Ev::metric).count(), 0);
}

/// ex04: same-span pre/post Peak/RMS with the shared measurement.
#[test]
#[expect(clippy::cast_precision_loss)]
fn pre_post_metrics_share_span_and_match_channel_level() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 1.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    let metrics: Vec<_> = evs.iter().filter_map(Ev::metric).collect();
    let (pre, post) = (metrics[0], metrics[1]);
    assert_eq!(pre.meta.generation, post.meta.generation);
    assert_eq!(pre.meta.sample_position, post.meta.sample_position);
    assert_eq!(pre.meta.node, c.proc);
    assert_eq!(pre.meta.port, c.proc_in);
    assert_eq!(post.meta.port, c.proc_out);
    // Input observed = materialized direct slot; output = ×2.
    assert!((post.levels[0].peak - pre.levels[0].peak * 2.0).abs() < 1e-6);
    assert!((post.levels[0].rms - pre.levels[0].rms * 2.0).abs() < 1e-6);
    // Independent check vs the shared measurement.
    let l: Vec<f32> = (0..FRAMES)
        .map(|i| (i as f32 + 1.0) / FRAMES as f32)
        .collect();
    let expect = ruviform_audio_analysis::ChannelLevel::measure(&l);
    assert!((pre.levels[0].peak - expect.peak).abs() < 1e-6);
    assert!((pre.levels[0].rms - expect.rms).abs() < 1e-6);
}

/// ex05 + ex06 + ex09: bounded capture arrives with full metadata,
/// budget stops to Monitor, explicit stop reports Requested.
#[test]
#[expect(clippy::cast_precision_loss)]
fn bounded_capture_metadata_and_stops() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let pc = ProbeConfig {
        pool_chunks: 4,
        chunk_frames: FRAMES,
        chunk_channels: 8,
        ..ProbeConfig::default()
    };
    let mut rig = rig(&c.spec, &pc, &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 3 * FRAMES as u64,
            },
        )
        .unwrap();

    let mut chunks = Vec::new();
    let mut stopped = Vec::new();
    for b in 0..5u32 {
        feed_ramp(&mut rig.rt, c.src_out, b as f32);
        rig.rt.process(FRAMES, &ctx()).unwrap();
        for e in collect(&mut rig) {
            match e {
                Ev::Cap {
                    meta,
                    frames,
                    channels,
                    data,
                } => {
                    chunks.push((meta, frames, channels, data));
                }
                Ev::Status(s) => stopped.push(s),
                Ev::Metric(_) => {}
            }
        }
    }
    assert_eq!(chunks.len(), 3, "budget caps capture at 3 blocks");
    for (i, (meta, frames, channels, _)) in chunks.iter().enumerate() {
        assert_eq!(meta.probe, p);
        assert_eq!(meta.node, c.proc);
        assert_eq!(meta.port, c.proc_out);
        assert_eq!(meta.generation, 1);
        assert_eq!(meta.sample_position, (i as u64) * FRAMES as u64);
        assert_eq!(*frames, FRAMES);
        assert_eq!(*channels, 2);
        assert_eq!(meta.sample_rate, 48_000);
        assert_eq!(meta.effective_latency, 0);
    }

    // Restart capture, stop explicitly after one block.
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 4 * FRAMES as u64,
            },
        )
        .unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 9.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    rig.broker.stop_capture(p).unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 10.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    assert!(evs.iter().filter_map(Ev::status).any(|s| {
        matches!(
            s.kind,
            StatusKind::CaptureStopped {
                reason: StopReason::Requested
            }
        )
    }));
    assert!(
        stopped.iter().any(|s| {
            matches!(
                s.kind,
                StatusKind::CaptureStopped {
                    reason: StopReason::BudgetExhausted
                }
            )
        }),
        "budget exhaustion returns the probe to Monitor"
    );
}

/// ex07: free-pool exhaustion drops blocks + counts, never blocks.
#[test]
fn free_pool_exhaustion_records_loss() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let pc = ProbeConfig {
        pool_chunks: 1,
        chunk_frames: FRAMES,
        chunk_channels: 8,
        ..ProbeConfig::default()
    };
    let mut rig = rig(&c.spec, &pc, &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 4 * FRAMES as u64,
            },
        )
        .unwrap();
    for _ in 0..3 {
        feed_ramp(&mut rig.rt, c.src_out, 0.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
    }
    let counts = rig.collector.counts();
    assert_eq!(counts.lost_blocks, 2, "one pooled chunk, two lost blocks");
    let evs = collect(&mut rig);
    assert!(
        evs.iter()
            .filter_map(Ev::status)
            .any(|s| { matches!(s.kind, StatusKind::LostBlocks) })
    );
}

/// ex08: telemetry overflow counts lost records but capture chunks
/// still arrive (channel separation).
#[test]
fn telemetry_overflow_drops_records_not_chunks() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let pc = ProbeConfig {
        pool_chunks: 8,
        chunk_frames: FRAMES,
        chunk_channels: 8,
        telemetry_capacity: 4,
        ..ProbeConfig::default()
    };
    let mut rig = rig(&c.spec, &pc, &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    // A second monitor floods telemetry so the tiny channel
    // overflows while capture keeps flowing on its own channel.
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 8 * FRAMES as u64,
            },
        )
        .unwrap();
    for _ in 0..8 {
        feed_ramp(&mut rig.rt, c.src_out, 0.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
    }
    assert!(rig.collector.counts().lost_records > 0);
    let chunks = collect(&mut rig)
        .iter()
        .filter(|e| matches!(e, Ev::Cap { .. }))
        .count();
    assert_eq!(chunks, 8, "capture unaffected by telemetry overflow");
}

/// ex10 + ex11: republish without rebind → Unbound guard blocks
/// stale observation; `on_publish` rebinds to the new generation.
#[test]
fn republish_rebind_guard() {
    let c = probe_chain();
    let mut f = MockFactory {
        reuse: true,
        ..MockFactory::default()
    };
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();

    // One block so the gen-1 binding is applied before republish.
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    assert_eq!(evs.iter().filter_map(Ev::metric).count(), 1);

    // Republish the same spec as generation 2 without on_publish.
    let compiler = Compiler::new(config(FRAMES));
    let plan_out = compiler
        .compile(
            &validate(spec),
            &mut f,
            rig.grt.registry(),
            rig.grt.allocator(),
            rig.grt.generation() + 1,
        )
        .unwrap();
    rig.grt.publish(plan_out).unwrap();

    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    assert!(
        evs.iter()
            .filter_map(Ev::status)
            .any(|s| { matches!(s.kind, StatusKind::Unbound) && s.probe == p }),
        "stale binding must surface Unbound, not wrong-slot data"
    );
    assert_eq!(
        evs.iter()
            .filter_map(Ev::metric)
            .filter(|m| m.meta.generation == 2)
            .count(),
        0,
        "no records under a mismatched generation"
    );

    // Rebind: Installed + gen-2 metrics resume.
    rig.broker.on_publish(&rig.grt, spec).unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    assert!(
        evs.iter()
            .filter_map(Ev::metric)
            .any(|m| { m.meta.generation == 2 && m.meta.probe == p })
    );
}

/// ex12: long collector stall does not block RT; unsubscribe keeps
/// the runtime running.
#[test]
fn stalled_and_dropped_collector_never_blocks() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let pc = ProbeConfig {
        pool_chunks: 2,
        chunk_frames: FRAMES,
        telemetry_capacity: 4,
        ..ProbeConfig::default()
    };
    let mut rig = rig(&c.spec, &pc, &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 8 * FRAMES as u64,
            },
        )
        .unwrap();
    for _ in 0..8 {
        feed_ramp(&mut rig.rt, c.src_out, 0.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
    }
    assert!(rig.collector.counts().lost_blocks > 0);
    // Unsubscribe: drop the collector; process keeps working.
    drop(std::mem::replace(
        &mut rig.collector,
        pair(&ProbeConfig::default()).collector,
    ));
    for _ in 0..2 {
        feed_ramp(&mut rig.rt, c.src_out, 0.0);
        rig.rt.process(FRAMES, &ctx()).unwrap();
    }
}

/// ex13: an un-fed producer slot is observed as Unobserved(Stale),
/// never served as data.
#[test]
fn unwritten_slot_is_unobserved_not_data() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    // Deliberately do not feed the boundary input.
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    assert!(evs.iter().filter_map(Ev::status).any(|s| {
        matches!(
            s.kind,
            StatusKind::Unobserved {
                reason: UnobservedReason::Stale
            }
        )
    }));
    assert_eq!(evs.iter().filter_map(Ev::metric).count(), 0);
}

/// ex14: every non-RT failure is a defined `ProbeError`.
#[test]
fn control_plane_errors_are_defined() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;

    // Unknown probe on every API — install+remove gives a real
    // ProbeId that no longer exists.
    let ghost = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker.remove(ghost).unwrap();
    assert!(matches!(
        rig.broker.set_mode(ghost, ProbeMode::Off),
        Err(ProbeError::UnknownProbe)
    ));
    assert!(matches!(
        rig.broker.capture(ghost, CaptureConfig { max_frames: 1 }),
        Err(ProbeError::UnknownProbe)
    ));
    assert!(matches!(
        rig.broker.stop_capture(ghost),
        Err(ProbeError::UnknownProbe)
    ));
    assert!(matches!(
        rig.broker.remove(ghost),
        Err(ProbeError::UnknownProbe)
    ));

    // Port belonging to a different node resolves `Unknown`.
    let err = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.sink_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        ProbeError::PointUnavailable {
            availability: PointAvailability::Unknown
        }
    ));

    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    let err = rig
        .broker
        .capture(p, CaptureConfig { max_frames: 0 })
        .unwrap_err();
    assert!(matches!(err, ProbeError::BudgetExceeded { .. }));
}

/// ex15: effective latency follows the per-tap rule on a PDC merge.
#[test]
fn effective_latency_reports_path_delay() {
    let m = merge();
    let mut f = MockFactory::default();
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    f.pending
        .insert(m.a, Box::new(MockDsp::new(2, 1.0, log.clone())));
    f.pending.insert(m.b, Box::new(MockDsp::new(10, 1.0, log)));
    let mut rig = rig(&m.spec, &ProbeConfig::default(), &mut f);
    let spec = &m.spec;
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: m.a,
                port: m.a_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    rig.broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: m.sink,
                port: m.sink_in,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    let ones = vec![1.0f32; FRAMES];
    feed(&mut rig.rt, m.src_a_out, &[&ones, &ones]);
    feed(&mut rig.rt, m.src_b_out, &[&ones, &ones]);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    let evs = collect(&mut rig);
    let lat = |port| {
        evs.iter()
            .filter_map(Ev::metric)
            .find(|m| m.meta.port == port)
            .map(|m| m.meta.effective_latency)
    };
    assert_eq!(lat(m.a_out), Some(2), "a_out taps after a (accum=2)");
    assert_eq!(
        lat(m.sink_in),
        Some(10),
        "graph output latency = total_latency"
    );
}

/// ex16: process allocates nothing in any probe mode once installed.
#[test]
fn rt_probe_process_allocates_nothing() {
    let c = probe_chain();
    let mut f = MockFactory::default();
    f.pending.insert(c.proc, gain2());
    let mut rig = rig(&c.spec, &ProbeConfig::default(), &mut f);
    let spec = &c.spec;
    let p = rig
        .broker
        .install(
            &rig.grt,
            spec,
            ProbePoint {
                node: c.proc,
                port: c.proc_out,
            },
            ProbeMode::Monitor,
        )
        .unwrap();
    // Apply the bind (one block outside the window).
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    rig.rt.process(FRAMES, &ctx()).unwrap();
    rig.collector.drain(|_| {});
    let rc = ctx();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    assert_no_alloc(|| rig.rt.process(FRAMES, &rc).unwrap());
    rig.collector.drain(|_| {});
    // Capture mode inside the window — chunk pool keeps it alloc-free.
    rig.broker
        .capture(
            p,
            CaptureConfig {
                max_frames: 4 * FRAMES as u64,
            },
        )
        .unwrap();
    rig.rt.process(FRAMES, &rc).unwrap();
    rig.collector.drain(|_| {});
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    assert_no_alloc(|| rig.rt.process(FRAMES, &rc).unwrap());
    rig.collector.drain(|_| {});
    // Off mode.
    rig.broker.set_mode(p, ProbeMode::Off).unwrap();
    rig.rt.process(FRAMES, &rc).unwrap();
    feed_ramp(&mut rig.rt, c.src_out, 0.0);
    assert_no_alloc(|| rig.rt.process(FRAMES, &rc).unwrap());
}
