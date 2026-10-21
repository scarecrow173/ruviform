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
//!   swap (see `docs/adr/audio-engine-snapshot-swap.md`)
//! - [`Session`] — owns the `OutputStream`; the device callback mints
//!   [`RtContext`] per block and drives the active plan
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

mod graph;
mod mixer;
mod plan;
mod session;

pub use graph::{AudioNode, Graph};
pub use mixer::Mixer;
pub use plan::{GraphPlan, PlanSwap, RtPlan};
pub use session::{Session, SessionError};

#[cfg(test)]
mod tests {
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

    #[cfg(debug_assertions)]
    mod alloc_probe {
        #[global_allocator]
        static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
    }

    /// Issue #22 lane: the RT callback path (plan lookup + mint + drive)
    /// must run with zero allocations.
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
}
