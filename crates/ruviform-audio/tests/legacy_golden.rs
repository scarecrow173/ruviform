//! G2-07 — legacy compatibility golden fixtures. The pre-M2 API
//! (`AudioNode`/`Graph`/`GraphPlan`/`PlanSwap`/`RtPlan`) must keep
//! producing the same observable output — these fixtures pin the
//! single-bus Main-only path semantics that migration must preserve
//! until removal approval.

use ruviform_audio::{AudioNode, Graph, GraphPlan, Mixer, PlanSwap};
use ruviform_audio_core::AudioBufferViewMut;
use ruviform_rt::RtContext;

/// Constant-fill source — the golden single-bus producer.
struct Fill(f32);

impl AudioNode for Fill {
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
        for ch in 0..out.channels() {
            let Some(plane) = out.channel_mut(ch) else {
                continue;
            };
            for s in plane.iter_mut() {
                *s = self.0;
            }
        }
    }
}

/// In-place gain — the golden single-bus transform.
struct Gain(f32);

impl AudioNode for Gain {
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
        for ch in 0..out.channels() {
            let Some(plane) = out.channel_mut(ch) else {
                continue;
            };
            for s in plane.iter_mut() {
                *s *= self.0;
            }
        }
    }
}

/// Golden: `Fill(1.0) → Gain(0.5) → Fill(+0.25, accumulate?)` —
/// the old ordered pull list renders in registration order, each
/// node seeing the prior node's output.
#[test]
fn legacy_single_bus_chain_golden() {
    let mut g = Graph::new();
    g.push(Box::new(Fill(1.0)));
    g.push(Box::new(Gain(0.5)));
    let mut buf = [0.0f32; 8]; // 2ch × 4f interleaved? No — planar view.
    let mut out = AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap();
    g.pull(&mut out, &RtContext::mint());
    // Fill writes 1.0 everywhere; Gain halves it → all 0.5.
    assert_eq!(buf, [0.5; 8]);
}

/// Golden: `GraphPlan::drive` over the same chain produces the same
/// values through the Mutex-wrapped plan as the bare graph.
#[test]
fn legacy_graph_plan_drive_golden() {
    let mut g = Graph::new();
    g.push(Box::new(Fill(2.0)));
    g.push(Box::new(Gain(0.25)));
    let plan = GraphPlan::new(g);
    let mut buf = [0.0f32; 8];
    plan.drive(
        &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
        &RtContext::mint(),
    );
    assert_eq!(buf, [0.5; 8]);
}

/// Golden: `PlanSwap` publishes the next plan on the next block —
/// before swap = old values, after = new values.
#[test]
fn legacy_plan_swap_golden() {
    let mut g = Graph::new();
    g.push(Box::new(Fill(1.0)));
    let (swap, mut rt) = PlanSwap::new(GraphPlan::new(g));
    let mut buf = [0.0f32; 8];
    rt.active().unwrap().drive(
        &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
        &RtContext::mint(),
    );
    assert_eq!(buf, [1.0; 8]);

    let mut g2 = Graph::new();
    g2.push(Box::new(Fill(3.0)));
    g2.push(Box::new(Gain(0.5)));
    swap.swap(GraphPlan::new(g2));
    rt.active().unwrap().drive(
        &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
        &RtContext::mint(),
    );
    assert_eq!(buf, [1.5; 8]);
}

/// Golden: the `Mixer` still sums planar inputs in place — the
/// single-bus merge semantic the typed runtime replaces for
/// multi-bus graphs.
#[test]
fn legacy_mixer_golden() {
    let mut mixer = Mixer::new(2, 2, 4);
    mixer.input_mut(0).unwrap().copy_from_slice(&[1.0; 8]);
    mixer.input_mut(1).unwrap().copy_from_slice(&[0.5; 8]);
    let mut buf = [0.0f32; 8];
    mixer.pull(
        &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
        &RtContext::mint(),
    );
    assert_eq!(buf, [1.5; 8]);
}

/// The `FnMut` closure node adhesion still works — the same wiring
/// pattern the old `PluginEndpoint` shell path relies on.
#[test]
fn legacy_closure_node_golden() {
    let mut g = Graph::new();
    g.push(Box::new(
        |out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext| {
            for ch in 0..out.channels() {
                if let Some(plane) = out.channel_mut(ch) {
                    plane.fill(0.75);
                }
            }
        },
    ));
    let mut buf = [0.0f32; 8];
    g.pull(
        &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
        &RtContext::mint(),
    );
    assert_eq!(buf, [0.75; 8]);
}
