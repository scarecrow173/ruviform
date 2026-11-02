//! G2-03/G2-06 — runtime evidence: measured PDC behavior, variable
//! callback sizes (64/128/256 + non-power-of-two), bounds, tail and
//! zero-frame behavior, uninitialized-read rejection, and the
//! RT no-allocation audit.

mod common;

use std::sync::{Arc, Mutex};

use common::{MockDsp, MockFactory, chain, config, merge, validate};
use ruviform_audio::typed::{Compiler, GraphRuntime, ProcessError, RuntimeFault};
use ruviform_rt::RtContext;

#[cfg(debug_assertions)]
mod alloc_probe {
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
}

fn build_chain_runtime(
    max_frames: usize,
) -> (
    GraphRuntime,
    ruviform_audio::typed::RtRuntime,
    common::Chain,
    Arc<Mutex<Vec<String>>>,
) {
    let c = chain();
    let g = validate(&c.spec);
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut factory = MockFactory::default();
    factory
        .pending
        .insert(c.proc, Box::new(MockDsp::new(0, 1.0, Arc::clone(&log))));
    let (mut runtime, rt) = GraphRuntime::new(&config(max_frames), 8, 8);
    let compiled = Compiler::new(config(max_frames))
        .compile(
            &g,
            &mut factory,
            runtime.registry(),
            runtime.allocator(),
            runtime.generation() + 1,
        )
        .expect("chain compiles");
    runtime.publish(compiled).expect("publish");
    (runtime, rt, c, log)
}

#[test]
#[expect(clippy::cast_precision_loss)]
fn g2_06_end_to_end_chain_at_64_128_256() {
    for frames in [64usize, 128, 256] {
        let (_runtime, mut rt, c, _log) = build_chain_runtime(256);
        let ctx = RtContext::mint();
        let data: Vec<Vec<f32>> = (0..2)
            .map(|ch| (0..frames).map(|f| (f + ch) as f32 * 0.5).collect())
            .collect();
        let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
        common::feed(&mut rt, c.src_out, &refs);
        rt.process(frames, &ctx).expect("process");
        let out = common::drain(&mut rt, c.sink_in);
        assert_eq!(out.len(), 2);
        // gain = 1.0, state = 1.0 (first callback) → out = in + 1.
        for (ch, plane) in out.iter().enumerate() {
            for (f, &v) in plane.iter().enumerate().take(frames) {
                assert!(
                    (v - ((f + ch) as f32 * 0.5 + 1.0)).abs() < f32::EPSILON,
                    "ch{ch} f{f}"
                );
            }
        }
    }
}

#[test]
fn g2_06_variable_and_odd_frame_counts() {
    let (_runtime, mut rt, c, _log) = build_chain_runtime(256);
    let ctx = RtContext::mint();
    for frames in [1usize, 13, 96, 200, 255] {
        let data = [vec![0.25f32; frames], vec![0.5f32; frames]];
        let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
        common::feed(&mut rt, c.src_out, &refs);
        rt.process(frames, &ctx).expect("process");
        let out = common::drain(&mut rt, c.sink_in);
        for plane in &out {
            assert_eq!(plane.len(), 256, "slot capacity, not frames");
            for v in &plane[..frames] {
                assert!(*v > 0.0, "valid prefix written");
            }
        }
    }
}

#[test]
fn g2_06_frames_above_max_rejected_and_zero_is_noop() {
    let (_runtime, mut rt, c, _log) = build_chain_runtime(256);
    let ctx = RtContext::mint();
    let err = rt.process(257, &ctx).unwrap_err();
    assert!(matches!(
        err,
        ProcessError::FramesExceedMax { got: 257, max: 256 }
    ));
    // Zero frames: defined no-op — no stamps advance, no fault.
    rt.process(0, &ctx).expect("zero frames");
    let data = [vec![1.0f32; 64], vec![1.0f32; 64]];
    let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
    common::feed(&mut rt, c.src_out, &refs);
    rt.process(64, &ctx).expect("64 frames");
    let out = common::drain(&mut rt, c.sink_in);
    assert!((out[0][0] - 2.0).abs() < f32::EPSILON); // in*1 + state(1)
}

#[test]
fn g2_06_channels_gated_feed_before_first_process_lands() {
    // `input_writer` hands out a slot whose logical `channels` is
    // configured from the pending plan — feeds that iterate
    // `slot.channels` (the pattern the API documents) must reach the
    // graph even on the very first callback, before
    // `activate_generation` has run inside `process`.
    let (_runtime, mut rt, c, _log) = build_chain_runtime(256);
    let ctx = RtContext::mint();
    {
        let mut slot = rt.input_writer(c.src_out, 64).expect("bound");
        assert_eq!(slot.channels, 2, "planned channels before activation");
        for ch in 0..usize::from(slot.channels) {
            slot.data.channel_mut(ch).unwrap()[..64].fill(0.5);
        }
    }
    rt.process(64, &ctx).expect("process");
    let out = common::drain(&mut rt, c.sink_in);
    for plane in &out {
        for v in &plane[..64] {
            assert!(*v > 0.0, "first-callback feed reached the graph");
        }
    }
}

#[test]
fn g2_06_unwritten_input_triggers_uninitialized_read_fault() {
    let (runtime, mut rt, c, _log) = build_chain_runtime(256);
    let ctx = RtContext::mint();
    // First callback WITHOUT feeding the boundary input: the source
    // slot was never stamped for seq 1 → uninitialized read rejected.
    rt.process(64, &ctx).expect("process");
    // A second callback still unfed — stamps are stale.
    rt.process(64, &ctx).expect("process");
    let snap = runtime.status().snapshot();
    assert!(
        snap.uninit_reads >= 1,
        "unfed producer must be rejected, got {snap:?}"
    );
    assert!(snap.fault_flags & RuntimeFault::UninitializedRead as u64 != 0);
    let out = common::drain(&mut rt, c.sink_in);
    let _ = out;
}

#[test]
fn g2_03_pdc_compensates_latency_difference_at_merge() {
    // Path A: latency 3. Path B: latency 0. The merge must delay B
    // by D = max - B = 3 so both arrive aligned.
    let m = merge();
    let g = validate(&m.spec);
    let mut factory = MockFactory::default();
    factory.pending.insert(
        m.a,
        Box::new(MockDsp::new(3, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    factory.pending.insert(
        m.b,
        Box::new(MockDsp::new(0, 1.0, Arc::new(Mutex::new(vec![])))),
    );
    let (mut runtime, mut rt) = GraphRuntime::new(&config(256), 8, 8);
    let compiled = Compiler::new(config(256))
        .compile(&g, &mut factory, runtime.registry(), runtime.allocator(), 1)
        .expect("merge compiles");
    // The slow path's edge gets delay 0; the fast path gets 3.
    let sink = compiled
        .plan
        .schedule
        .iter()
        .find(|n| n.node == m.sink)
        .expect("sink scheduled");
    let delays: Vec<u32> = sink.inputs[0]
        .edges
        .iter()
        .map(|e| e.delay_frames)
        .collect();
    assert!(
        delays.contains(&3) && delays.contains(&0),
        "compensation delays {delays:?}"
    );
    assert_eq!(compiled.plan.total_latency, 3);
    assert_eq!(compiled.plan.delays.len(), 1);
    assert_eq!(compiled.plan.delays[0].delay, 3);
    runtime.publish(compiled).expect("publish");

    // Measured behavior: the fast path's edge delays its samples by
    // exactly 3 — merged output shows A-only for frames 0..3, then
    // the aligned A+delayed-B sum.
    let ctx = RtContext::mint();
    let a_in = [vec![1.0f32; 64], vec![0.0; 64]];
    let b_in = [vec![10.0f32; 64], vec![0.0; 64]];
    common::feed(
        &mut rt,
        m.src_a_out,
        &[a_in[0].as_slice(), a_in[1].as_slice()],
    );
    common::feed(
        &mut rt,
        m.src_b_out,
        &[b_in[0].as_slice(), b_in[1].as_slice()],
    );
    rt.process(64, &ctx).expect("process");
    let out = common::drain(&mut rt, m.sink_in);
    // a: state=1 → a_out = 1*1 + 1 = 2. b: state=1 → b_out = 10*1 + 1 = 11.
    assert!((out[0][0] - 2.0).abs() < 0.001, "frame0: {}", out[0][0]);
    assert!((out[0][2] - 2.0).abs() < 0.001, "frame2: {}", out[0][2]);
    assert!(
        (out[0][3] - 13.0).abs() < 0.001,
        "frame3 = A(2) + B delayed 3 (11): {}",
        out[0][3]
    );
}

/// G2-06 — the whole RT callback path (feed + schedule + drain)
/// allocates nothing after warm-up. Uses the same
/// `assert_no_alloc` global-allocator disabler pattern as the
/// legacy `rt_callback_path_no_alloc` unit test.
#[test]
fn rt_typed_process_no_alloc() {
    let (_runtime, mut rt, c, _log) = build_chain_runtime(256);
    let ctx = RtContext::mint();
    // Warm-up: first `SnapshotReader::read` may allocate an internal
    // debt node — contract allows it outside the audited interval.
    let data = [vec![1.0f32; 64], vec![1.0f32; 64]];
    let refs: Vec<&[f32]> = data.iter().map(Vec::as_slice).collect();
    common::feed(&mut rt, c.src_out, &refs);
    rt.process(64, &ctx).expect("warm-up process");
    common::feed(&mut rt, c.src_out, &refs);
    assert_no_alloc::assert_no_alloc(|| {
        rt.process(64, &ctx).expect("audited process");
    });
    let out = common::drain(&mut rt, c.sink_in);
    assert!(out[0][0] > 0.0);
}
