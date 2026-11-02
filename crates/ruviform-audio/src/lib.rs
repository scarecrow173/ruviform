//! `ruviform-audio` — the audio engine: pull-based processing graph,
//! planar-f32 mixer, and device-session mediation (Issue #25).
//!
//! Layers the RT output path over the `ruviform-audio-core` device port
//! traits ([`ruviform_audio_core::device`]) and the `ruviform-rt`
//! `Processor`/`RtContext` contract:
//!
//! - [`Graph`] — ordered `AudioNode` list, pull-driven per block
//! - [`Mixer`] — planar f32 summing node
//! - [`GraphPlan`] / [`PlanSwap`] — immutable snapshot + `RwLock`/`try_read`
//!   swap (see `docs/adr/audio-engine-snapshot-swap.md`), with retired
//!   plans deferred to the non-RT side via the `ruviform-rt` retire
//!   channel ([`PlanSwap::drain_retired`]/[`PlanSwap::retired_overflows`])
//! - [`Session`] — owns the `OutputStream`; the device callback mints
//!   [`RtContext`](ruviform_rt::RtContext) per block and drives the
//!   active plan
//!
//! ## Allowed dependency layers
//!
//! Domain foundations (`ruviform-audio-core`) + Primitives
//! (`ruviform-rt`). `ruviform-audio-device` is an adapter — the engine
//! never names its types; the session speaks only the port traits.
//!
//! ## RT constraints
//!
//! The device callback path ([`Session`]'s internal callback and every
//! `AudioNode::pull` it drives) runs on the backend RT thread:
//! allocation, blocking and OS calls are contract violations. All
//! construction/mutation APIs are `non-RT only`.

mod denormal;
mod graph;
mod mixer;
mod plan;
pub mod render;
mod session;
pub mod typed;

pub use denormal::DenormalFlush;
pub use graph::{AudioNode, Graph};
pub use mixer::Mixer;
pub use plan::{GraphPlan, PlanSwap, RtPlan};
pub use session::{Session, SessionError};

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use ruviform_audio_core::device::{
        BufferSize, Device, DeviceResult, OutputCallbackInfo, OutputDataCallback,
        OutputStreamConfig, OutputTimestamp,
    };
    use ruviform_audio_core::{
        AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
    };
    use ruviform_media_core::MediaTime;
    use ruviform_rt::RtContext;

    use super::*;

    // Compile-time thread-boundary evidence (Issue #113 T5): the types
    // that cross to the callback thread — `RtPlan` itself and the
    // `GraphPlan`s it retires — are `Send` structurally (no `unsafe
    // impl Send` anywhere); the `DropCollector` the `Retirer` feeds
    // must receive `Arc<GraphPlan>`s across threads, so `GraphPlan`
    // must be `Send + Sync`. `static_assertions` covers the negative
    // side that plain `expect_send`-style functions cannot express.
    static_assertions::assert_impl_all!(GraphPlan: Send, Sync);
    static_assertions::assert_impl_all!(RtPlan: Send);
    static_assertions::assert_impl_all!(PlanSwap: Send, Sync);
    static_assertions::assert_impl_all!(Session: Send);
    static_assertions::assert_impl_all!(OutputDataCallback: Send);
    // Negative-side pin: the canonical `!Send`/`!Sync` payloads a
    // closure capture must refuse.
    static_assertions::assert_not_impl_any!(std::rc::Rc<u8>: Send, Sync);
    static_assertions::assert_not_impl_any!(std::cell::RefCell<u8>: Sync);

    /// Records every `pull` into a shared log — verifies call order.
    struct TraceNode {
        tag: u8,
        log: Arc<Mutex<Vec<u8>>>,
    }
    impl AudioNode for TraceNode {
        fn pull(&mut self, _out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
            self.log.lock().unwrap().push(self.tag);
        }
    }

    /// Writes a constant into every frame of `out`.
    struct FillNode(f32);
    impl AudioNode for FillNode {
        fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
            for ch in 0..out.channels() {
                if let Some(plane) = out.channel_mut(ch) {
                    plane.fill(self.0);
                }
            }
        }
    }

    struct MockStream {
        format: AudioFormat,
    }
    impl ruviform_audio_core::device::OutputStream for MockStream {
        fn start(&self) -> DeviceResult<()> {
            Ok(())
        }
        fn stop(&self) -> DeviceResult<()> {
            Ok(())
        }
        fn format(&self) -> AudioFormat {
            self.format
        }
        fn buffer_size(&self) -> DeviceResult<BufferSize> {
            Ok(BufferSize::Fixed(256))
        }
        fn now(&self) -> DeviceResult<MediaTime> {
            Ok(MediaTime::new(
                0,
                ruviform_audio_core::device::DEVICE_CLOCK_TIMEBASE,
            ))
        }
    }

    /// Captures the data callback so tests can fire it manually.
    struct MockDevice {
        format: AudioFormat,
        callback: Arc<Mutex<Option<OutputDataCallback>>>,
    }
    impl MockDevice {
        fn new() -> (Self, Arc<Mutex<Option<OutputDataCallback>>>) {
            let slot = Arc::new(Mutex::new(None));
            let dev = Self {
                format: AudioFormat {
                    sample_rate: SampleRate::new(48_000).unwrap(),
                    layout: ChannelLayout::Stereo,
                    sample_format: SampleFormat::F32,
                },
                callback: Arc::clone(&slot),
            };
            (dev, slot)
        }
    }
    impl Device for MockDevice {
        fn name(&self) -> DeviceResult<String> {
            Ok("mock".into())
        }
        fn id(&self) -> DeviceResult<Option<ruviform_audio_core::device::DeviceId>> {
            Ok(None)
        }
        fn default_output_format(&self) -> DeviceResult<AudioFormat> {
            Ok(self.format)
        }
        fn supported_output_formats(
            &self,
        ) -> DeviceResult<Vec<ruviform_audio_core::device::SupportedOutputFormat>> {
            Ok(Vec::new())
        }
        fn open_output_stream(
            &self,
            config: &OutputStreamConfig,
            data_callback: OutputDataCallback,
            _error_callback: ruviform_audio_core::device::OutputErrorCallback,
        ) -> DeviceResult<Box<dyn ruviform_audio_core::device::OutputStream>> {
            *self.callback.lock().unwrap() = Some(data_callback);
            Ok(Box::new(MockStream {
                format: config.format,
            }))
        }
    }

    fn config() -> OutputStreamConfig {
        OutputStreamConfig {
            format: AudioFormat {
                sample_rate: SampleRate::new(48_000).unwrap(),
                layout: ChannelLayout::Stereo,
                sample_format: SampleFormat::F32,
            },
            buffer_size: BufferSize::Default,
        }
    }

    fn info() -> OutputCallbackInfo {
        OutputCallbackInfo {
            timestamp: OutputTimestamp {
                callback: MediaTime::new(0, ruviform_audio_core::device::DEVICE_CLOCK_TIMEBASE),
                playback: None,
            },
        }
    }

    #[test]
    fn graph_pull_drives_nodes_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut g = Graph::new();
        for tag in [7u8, 3, 5] {
            g.push(Box::new(TraceNode {
                tag,
                log: Arc::clone(&log),
            }));
        }
        let mut buf = [0.0f32; 8];
        let mut out = AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap();
        g.pull(&mut out, &RtContext::mint());
        assert_eq!(*log.lock().unwrap(), vec![7, 3, 5]);
    }

    #[test]
    fn mixer_sums_planar_channels() {
        let mut mixer = Mixer::new(2, 2, 4);
        mixer.input_mut(0).unwrap().copy_from_slice(&[1.0; 8]);
        mixer.input_mut(1).unwrap().copy_from_slice(&[0.5; 8]);
        let mut buf = [0.0f32; 8];
        let mut out = AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap();
        mixer.pull(&mut out, &RtContext::mint());
        assert_eq!(buf, [1.5; 8]);
    }

    #[test]
    fn plan_swap_applies_next_block() {
        let mut g = Graph::new();
        g.push(Box::new(FillNode(1.0)));
        let (swap, mut rt) = PlanSwap::new(GraphPlan::new(g));
        let mut buf = [0.0f32; 8];
        rt.active().unwrap().drive(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &RtContext::mint(),
        );
        assert_eq!(buf, [1.0; 8]);

        let mut g2 = Graph::new();
        g2.push(Box::new(FillNode(9.0)));
        swap.swap(GraphPlan::new(g2));
        let mut buf = [0.0f32; 8];
        rt.active().unwrap().drive(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &RtContext::mint(),
        );
        assert_eq!(buf, [9.0; 8]);
    }

    #[test]
    fn session_drives_plan_through_callback() {
        let (device, slot) = MockDevice::new();
        let mut g = Graph::new();
        g.push(Box::new(FillNode(0.25)));
        let session = Session::open(&device, config(), GraphPlan::new(g)).unwrap();
        session.start().unwrap();

        let mut cb = slot.lock().unwrap().take().unwrap();
        let mut buf = [0.0f32; 8];
        cb(AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(), info());
        assert_eq!(buf, [0.25; 8]);

        // Swap mid-stream: next callback uses the new plan.
        let mut g2 = Graph::new();
        g2.push(Box::new(FillNode(0.5)));
        session.swap(GraphPlan::new(g2));
        let mut buf = [0.0f32; 8];
        cb(AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(), info());
        assert_eq!(buf, [0.5; 8]);
    }

    #[test]
    fn denormal_flush_zeroes_denormals() {
        let den = f32::MIN_POSITIVE / 2.0;
        let mut buf = [den, -den, 1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, -0.5];
        DenormalFlush.pull(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &RtContext::mint(),
        );
        assert_eq!(buf[0], 0.0);
        assert!(buf[1].is_sign_negative() && buf[1] == 0.0);
        assert_eq!(buf[2], 1.0);
        assert_eq!(buf[3], 0.0);
        assert!(buf[4].is_sign_negative());
        assert!(buf[5].is_nan());
        assert_eq!(buf[6], f32::INFINITY);
        assert_eq!(buf[7], -0.5);
    }

    struct EmitDenormal;
    impl AudioNode for EmitDenormal {
        fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
            for ch in 0..out.channels() {
                if let Some(plane) = out.channel_mut(ch) {
                    plane.fill(f32::MIN_POSITIVE / 2.0);
                }
            }
        }
    }

    struct ReadFirst;
    impl AudioNode for ReadFirst {
        fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {
            assert_eq!(out.channel(0).unwrap()[0], 0.0);
        }
    }

    #[test]
    fn denormal_flush_in_graph_order() {
        // Flush sits between nodes — only the preceding node's output
        // is normalized before the next node runs.
        let mut g = Graph::new();
        g.push(Box::new(EmitDenormal));
        g.push(Box::new(DenormalFlush));
        g.push(Box::new(ReadFirst));
        let mut buf = [0.0f32; 8];
        g.pull(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &RtContext::mint(),
        );
        assert_eq!(buf, [0.0; 8]);
    }

    #[test]
    fn rt_denormal_flush_no_alloc() {
        let den = f32::MIN_POSITIVE / 2.0;
        let mut buf = [den, -den, 1.0, 2.0, den, -den, 0.5, -0.5];
        let mut out = AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap();
        let ctx = RtContext::mint();
        assert_no_alloc::assert_no_alloc(|| {
            DenormalFlush.pull(&mut out, &ctx);
        });
        assert_eq!(buf[0], 0.0);
        assert_eq!(buf[2], 1.0);
    }

    #[cfg(debug_assertions)]
    mod alloc_probe {
        #[global_allocator]
        static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
    }

    /// Issue #22 lane: the RT callback path (plan lookup + mint + drive)
    /// must run with zero allocations — on a *cold* plan, so the first
    /// `drive` after construction is covered as well (Issue #214:
    /// platforms whose `Mutex` allocates at first lock rely on
    /// `GraphPlan::new` having materialized it).
    #[test]
    fn rt_callback_path_no_alloc() {
        let (device, _slot) = MockDevice::new();
        let mut g = Graph::new();
        g.push(Box::new(FillNode(0.25)));
        let session = Session::open(&device, config(), GraphPlan::new(g)).unwrap();

        // Rebuild the same wiring the callback uses.
        let (_swap, mut rt) = PlanSwap::new(GraphPlan::new({
            let mut g = Graph::new();
            g.push(Box::new(FillNode(1.0)));
            g
        }));
        let mut buf = [0.0f32; 8];
        assert_no_alloc::assert_no_alloc(|| {
            let plan = rt.active().unwrap();
            let ctx = RtContext::mint();
            plan.drive(
                &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
                &ctx,
            );
        });
        let _ = session;
    }

    /// Node whose `Drop` flips a shared flag — probes *when and where*
    /// a retired `GraphPlan`'s destructor chain runs.
    struct DropFlagNode(Arc<AtomicBool>);
    impl AudioNode for DropFlagNode {
        fn pull(&mut self, _out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext) {}
    }
    impl Drop for DropFlagNode {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// A `GraphPlan` whose single node reports its drop via `flag`.
    fn flagged_plan(flag: &Arc<AtomicBool>) -> GraphPlan {
        let mut g = Graph::new();
        g.push(Box::new(DropFlagNode(Arc::clone(flag))));
        GraphPlan::new(g)
    }

    /// The `FnMut` blanket impl: a `Send` closure coerces to
    /// `Box<dyn AudioNode>` through `Graph::push` and its body runs on
    /// `pull`. (Issue #113 T5 — closure adhesion, no named newtype.)
    #[test]
    fn fn_mut_closure_node_pushes_and_pulls() {
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&hits);
        let mut g = Graph::new();
        g.push(Box::new(
            move |out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext| {
                counted.fetch_add(1, Ordering::Relaxed);
                if let Some(plane) = out.channel_mut(0) {
                    plane.fill(1.0);
                }
            },
        ));
        let mut buf = [0.0f32; 8];
        g.pull(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &RtContext::mint(),
        );
        assert_eq!(hits.load(Ordering::Relaxed), 1);
        assert_eq!(buf[..4], [1.0; 4]);
        assert_eq!(buf[4..], [0.0; 4]);
    }

    /// RT contract: `RtPlan::active` re-reading the same published
    /// `Arc` never retires and allocates nothing — `ptr_eq` skips the
    /// retire push, and the collector stays empty.
    #[test]
    fn rt_plan_active_no_alloc() {
        let (swap, mut rt) = PlanSwap::new(GraphPlan::new(Graph::new()));
        // Warm-up outside the window: first read fills the cache.
        // `active` returns a borrow of `cached`, not an `Arc` — pin
        // the borrow against the slot's own `Arc` (`Arc::as_ptr`),
        // and keep only raw pointers so sequential `active()` calls
        // can coexist (a held borrow blocks the next `&mut self`
        // call by design).
        let first = std::ptr::from_ref(rt.active().unwrap());
        assert!(std::ptr::eq(
            first,
            Arc::as_ptr(rt.cached.as_ref().unwrap())
        ));
        assert_no_alloc::assert_no_alloc(|| {
            let a = std::ptr::from_ref(rt.active().unwrap());
            let b = std::ptr::from_ref(rt.active().unwrap());
            // Same-plan re-reads: every borrow points at the cached
            // slot's `Arc` pointee.
            assert_eq!(a, first);
            assert_eq!(b, first);
            assert!(std::ptr::eq(a, Arc::as_ptr(rt.cached.as_ref().unwrap())));
        });
        // Same-Arc re-reads retired nothing.
        assert_eq!(swap.drain_retired(), 0);
        assert_eq!(swap.retired_overflows(), 0);
    }

    /// RT contract: after `swap` publishes a different plan, `active`
    /// pushes the displaced `cached` `Arc` to the retire channel — the
    /// `GraphPlan` destructor does not run on the RT path, only on the
    /// non-RT `drain_retired`.
    #[test]
    fn rt_plan_swap_retire_no_alloc() {
        let flag = Arc::new(AtomicBool::new(false));
        let (swap, mut rt) = PlanSwap::new(flagged_plan(&flag));
        // Caches plan A — keep only the raw pointer so the next
        // `active()` borrow is free to displace it.
        let plan_a = std::ptr::from_ref(rt.active().unwrap());
        swap.swap(GraphPlan::new(Graph::new())); // publishes plan B
        assert_no_alloc::assert_no_alloc(|| {
            // Displaces + retires A: the returned borrow now points
            // at the cached slot's new `Arc` (plan B), not A.
            let active = std::ptr::from_ref(rt.active().unwrap());
            assert_ne!(active, plan_a);
            assert!(std::ptr::eq(
                active,
                Arc::as_ptr(rt.cached.as_ref().unwrap())
            ));
        });
        // Retired means queued: the destructor has not run yet.
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(swap.drain_retired(), 1);
        assert!(flag.load(Ordering::SeqCst));
        assert_eq!(swap.retired_overflows(), 0);
    }

    /// RT contract: `Drop for RtPlan` retires the still-cached plan
    /// inside the no-alloc window — push only, no destructor on the
    /// dropping side, no touch of the shared `current` slot.
    #[test]
    fn rt_plan_drop_retire_no_alloc() {
        let flag = Arc::new(AtomicBool::new(false));
        let (swap, mut rt) = PlanSwap::new(flagged_plan(&flag));
        rt.active().unwrap();
        // Publish a different plan first so the retired `cached` is the
        // last `Arc` to plan A — while the shared slot still publishes
        // A, a drain can only drop the queued `Arc` (refcount), not the
        // still-live plan.
        swap.swap(GraphPlan::new(Graph::new()));
        assert_no_alloc::assert_no_alloc(move || drop(rt));
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(swap.drain_retired(), 1);
        assert!(flag.load(Ordering::SeqCst));
        assert_eq!(swap.retired_overflows(), 0);
    }

    /// RT contract: the `FnMut`-node pull path — `active` → `drive` →
    /// closure — allocates nothing. Fixture and warm-up live outside
    /// the measured window.
    #[test]
    fn rt_fn_node_pull_no_alloc() {
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&hits);
        let mut g = Graph::new();
        g.push(Box::new(
            move |out: &mut AudioBufferViewMut<'_, f32>, _ctx: &RtContext| {
                counted.fetch_add(1, Ordering::Relaxed);
                for ch in 0..out.channels() {
                    if let Some(plane) = out.channel_mut(ch) {
                        plane.fill(0.5);
                    }
                }
            },
        ));
        let (_swap, mut rt) = PlanSwap::new(GraphPlan::new(g));
        let ctx = RtContext::mint();
        let mut buf = [0.0f32; 8];
        // Warm-up outside the window: first `active` read + first pull.
        rt.active().unwrap().drive(
            &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
            &ctx,
        );
        assert_no_alloc::assert_no_alloc(|| {
            let plan = rt.active().unwrap();
            plan.drive(
                &mut AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(),
                &ctx,
            );
        });
        assert_eq!(hits.load(Ordering::Relaxed), 2);
        assert_eq!(buf, [0.5; 8]);
    }

    /// Retirement lands on the non-RT drain, not on the RT path —
    /// and a cloned `PlanSwap` handle shares the same collector.
    /// (Named to keep `rt_` out of the RT-contract test lane.)
    #[test]
    fn plan_retirement_drops_via_collector_drain() {
        let flag = Arc::new(AtomicBool::new(false));
        let (swap, mut rt) = PlanSwap::with_retire_capacity(flagged_plan(&flag), 2);
        rt.active().unwrap();
        swap.swap(GraphPlan::new(Graph::new()));
        rt.active().unwrap(); // retires plan A — destructor deferred
        assert!(!flag.load(Ordering::SeqCst));

        // `swap` itself drains before publishing: plan A is reclaimed
        // on this non-RT call.
        let drain_handle = swap.clone();
        drain_handle.swap(GraphPlan::new(Graph::new()));
        assert!(flag.load(Ordering::SeqCst));
        assert_eq!(swap.drain_retired(), 0);
        assert_eq!(drain_handle.retired_overflows(), 0);
    }

    /// Teardown order recovers retired plans without any `swap`:
    /// `RtPlan::drop` queues `cached` (flag stays down), and the
    /// collector's own drop — non-RT side per the `Session` field
    /// order — runs the queued destructor on this thread.
    #[test]
    fn plan_teardown_recovers_retired_without_swap() {
        let flag = Arc::new(AtomicBool::new(false));
        let (swap, mut rt) = PlanSwap::new(flagged_plan(&flag));
        rt.active().unwrap();
        drop(rt);
        assert!(!flag.load(Ordering::SeqCst));
        drop(swap);
        assert!(flag.load(Ordering::SeqCst));
    }

    /// `Session` exposes the retirement observation points: swap →
    /// callback retire → `drain_retired` → `retired_overflows`, and a
    /// session that never swaps still recovers the cached plan at
    /// teardown (callback/`RtPlan` drop → collector/ring teardown on
    /// the dropping thread).
    #[test]
    fn session_retire_passthrough_and_teardown() {
        let flag_a = Arc::new(AtomicBool::new(false));
        let flag_b = Arc::new(AtomicBool::new(false));
        let (device, slot) = MockDevice::new();
        let session = Session::open(&device, config(), flagged_plan(&flag_a)).unwrap();

        let mut cb = slot.lock().unwrap().take().unwrap();
        let mut buf = [0.0f32; 8];
        cb(AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(), info()); // caches plan A

        session.swap(flagged_plan(&flag_b));
        let mut buf = [0.0f32; 8];
        cb(AudioBufferViewMut::try_new(&mut buf, 2, 4).unwrap(), info()); // active() reads B, retires A — push only
        assert!(!flag_a.load(Ordering::SeqCst));

        assert_eq!(session.drain_retired(), 1);
        assert!(flag_a.load(Ordering::SeqCst));
        assert_eq!(session.retired_overflows(), 0);

        // No further swap: dropping the callback drops `RtPlan`, whose
        // `Drop` queues plan B; the session's collector still lives,
        // so B's destructor waits for session teardown on this thread.
        drop(cb);
        assert!(!flag_b.load(Ordering::SeqCst));
        drop(session);
        assert!(flag_b.load(Ordering::SeqCst));
    }
}
