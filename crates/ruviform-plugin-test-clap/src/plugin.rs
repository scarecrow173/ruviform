//! The `clack` wiring layer: plugin descriptor, factory entry types,
//! and the thin callback shell that drives the [`block`] translation
//! layer — no binding logic lives here that cannot be exercised
//! through `block`'s testable functions.
//!
//! ## Lifecycle mapping (CLAP → neutral [`LifecycleState`])
//!
//! | CLAP callback | Neutral event |
//! | --- | --- |
//! | `new_shared` (`plugin.init`) | `Initialize` |
//! | `new_main_thread` failure | `InitFailed` |
//! | `activate` success | `Activate`; validation/prepare failure → `ActivateFailed` |
//! | `start_processing` | `StartProcessing` |
//! | `stop_processing` | `StopProcessing` |
//! | `process` | `Process` (self-loop gate — illegal outside `Processing`) |
//! | `deactivate` | `Deactivate` |
//! | `Shared::drop` (`plugin.destroy`) | `Destroy` |
//! | `reset` | no transition — resyncs the processor from the param mirror |
//!
//! CLAP gives no user `init`/`destroy` body beyond instance
//! construction and `Drop`, so the shared-state constructor and its
//! `Drop` impl are the hooks. `stop_processing`/`deactivate`/`destroy`
//! have no error channel — a host protocol violation there is
//! recorded as a rejected transition (the machine stays honest) and
//! the instance's remaining callbacks fail closed.
//!
//! ## Parameter mirror
//!
//! The gain value lives in two places per CLAP's thread model: the
//! audio processor's [`TestGainProcessor`] (RT path) and an
//! `AtomicU64` mirror in [`TestGainShared`] (`params` extension
//! queries, `flush`). Each `process` block syncs local state from
//! the mirror, applies sample-accurate events, and stores the
//! resulting value back — three atomic ops per block, all
//! allocation-free. `activate` resets both sides to the parameter
//! default (the fixture's deterministic per-activation law).

use core::fmt::Write as _;
use std::ffi::CStr;

use clack_common::events::io::{InputEvents, OutputEvents};
use clack_common::plugin::features;
use clack_common::utils::{ClapId, Cookie};
use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_extensions::params::{
    ParamDisplayWriter, ParamInfoFlags, ParamInfoWriter, PluginAudioProcessorParams,
    PluginMainThreadParams, PluginParams,
};
use clack_plugin::extensions::PluginExtensions;
use clack_plugin::host::{HostAudioProcessorHandle, HostMainThreadHandle, HostSharedHandle};
use clack_plugin::plugin::{
    Plugin, PluginAudioProcessor, PluginDescriptor, PluginError, PluginMainThread, PluginShared,
};
use clack_plugin::process::{
    Audio, Events, PluginAudioConfiguration, Process, ProcessStatus as ClapProcessStatus,
};

use ruviform_plugin_core::{
    EventKind, LifecycleEvent, PluginProcessor, ProcessConfig, ProcessContext,
};

use crate::block::{self, AtomicLifecycle, BlockScratch, BlockViews, ParamMirror};
use crate::gain::{GAIN_PARAM, TestGainProcessor};

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the gain parameter mirror.
pub(crate) struct TestGainShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The gain parameter's current plain value, mirrored for the
    /// main-thread `params` extension.
    gain_mirror: ParamMirror,
}

impl PluginShared<'_> for TestGainShared {}

impl TestGainShared {
    /// The `plugin.init` hook's fallible core: Created → Initialize
    /// is unconditional for this stateless init.
    ///
    /// **RT contract:** non-RT only — runs at instance creation.
    ///
    /// # Errors
    ///
    /// `PluginError::Message` — the lifecycle machine rejected
    /// `Initialize` (a re-init host protocol violation).
    fn initialize() -> Result<Self, PluginError> {
        let shared = Self {
            lifecycle: AtomicLifecycle::new(),
            gain_mirror: ParamMirror::new(GAIN_PARAM.default_value),
        };
        shared
            .lifecycle
            .transition(LifecycleEvent::Initialize)
            .map_err(|_| PluginError::Message("init outside Created state"))?;
        Ok(shared)
    }

    /// The `plugin.destroy` transition — invoked from [`Drop`]; kept
    /// as one authoritative body so tests can observe the terminal
    /// state the destructor records (a dropped value cannot be
    /// inspected afterwards).
    fn destroy(&self) {
        // `plugin.destroy` — legal from Created/Initialized only; a
        // host destroying an activated/processing instance is a
        // protocol violation: the transition is *rejected* (the
        // machine does not silently fabricate a Deactivate), and the
        // instance then dies with the honest state recorded.
        let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
    }
}

impl Drop for TestGainShared {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port and
/// parameter metadata live here.
pub(crate) struct TestGainMainThread<'a> {
    shared: &'a TestGainShared,
}

impl<'a> PluginMainThread<'a, TestGainShared> for TestGainMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`TestGainProcessor`], its context and all per-block scratch.
pub(crate) struct TestGainAudioProcessor<'a> {
    shared: &'a TestGainShared,
    processor: TestGainProcessor,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `test_gain` plugin type — a unit struct tying the associated
/// types together, exported through `SinglePluginEntry`.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TestGainClapPlugin;

impl Plugin for TestGainClapPlugin {
    type AudioProcessor<'a> = TestGainAudioProcessor<'a>;
    type Shared<'a> = TestGainShared;
    type MainThread<'a> = TestGainMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&TestGainShared>) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for TestGainClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform-test.clap-test-gain", "test_gain")
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — deterministic gain for host verification",
            )
            .with_features([features::AUDIO_EFFECT, features::STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<TestGainShared, PluginError> {
        // `plugin.init` — the CLAP init hook runs the shared/main
        // constructors. The host handle is intentionally not stored:
        // this fixture declares no host-facing extensions (no
        // thread-check, no GUI, no state), so nothing needs it after
        // construction.
        TestGainShared::initialize()
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a TestGainShared,
    ) -> Result<TestGainMainThread<'a>, PluginError> {
        // Nothing fallible here — but the failure arm documents the
        // `InitFailed` mapping for future fallible init work.
        Ok(TestGainMainThread { shared })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for TestGainMainThread<'_> {
    fn count(&self, _is_input: bool) -> u32 {
        1
    }

    fn get(&self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index != 0 {
            return;
        }
        writer.set(&AudioPortInfo {
            // Both directions share id 0 — legal per CLAP (ids must be
            // unique *within* a direction) and required for
            // `in_place_pair` pairing.
            id: ClapId::new(0),
            name: if is_input {
                b"Audio In" as &[u8]
            } else {
                b"Audio Out" as &[u8]
            },
            channel_count: 2,
            flags: AudioPortFlags::IS_MAIN,
            port_type: Some(AudioPortType::STEREO),
            in_place_pair: Some(ClapId::new(0)),
        });
    }
}

// ---------------------------------------------------------------------------
// Params extension
// ---------------------------------------------------------------------------

impl PluginMainThreadParams for TestGainMainThread<'_> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }
        info.set(&clack_extensions::params::ParamInfo {
            id: ClapId::new(GAIN_PARAM.id.0),
            flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::REQUIRES_PROCESS,
            cookie: Cookie::empty(),
            name: GAIN_PARAM.name.as_bytes(),
            module: b"/",
            min_value: GAIN_PARAM.min_value,
            max_value: GAIN_PARAM.max_value,
            default_value: GAIN_PARAM.default_value,
        });
    }

    fn get_value(&self, param_id: ClapId) -> Option<f64> {
        (param_id.get() == GAIN_PARAM.id.0).then(|| self.shared.gain_mirror.load())
    }

    fn value_to_text(
        &self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        if param_id.get() != GAIN_PARAM.id.0 {
            return Err(core::fmt::Error);
        }
        write!(writer, "{value:.4}x")
    }

    fn text_to_value(&self, param_id: ClapId, text: &CStr) -> Option<f64> {
        if param_id.get() != GAIN_PARAM.id.0 {
            return None;
        }
        let text = text.to_str().ok()?.trim().trim_end_matches('x').trim();
        let value: f64 = text.parse().ok()?;
        // `clamp` propagates NaN — a non-finite parse is rejected,
        // never handed to the host as a value.
        value
            .is_finite()
            .then(|| value.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Main-thread flush (e.g. changes applied while inactive):
        // fold every ParamValue for our param into the mirror;
        // offsets carry no meaning outside a process block. The scan
        // is bounded like the process path's.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &GAIN_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .gain_mirror
                    .store(GAIN_PARAM.denormalize(param_event.value));
            }
        }
    }
}

impl PluginAudioProcessorParams for TestGainAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Audio-thread flush: mirror the change and resync the live
        // processor — CLAP does not guarantee offsets here. The scan
        // is bounded like the process path's.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &GAIN_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .gain_mirror
                    .store(GAIN_PARAM.denormalize(param_event.value));
            }
        }
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
    }
}

// ---------------------------------------------------------------------------
// Audio processor
// ---------------------------------------------------------------------------

impl<'a> TestGainAudioProcessor<'a> {
    /// The body of [`PluginAudioProcessor::activate`], minus the host
    /// handles the trait signature carries (unused by this fixture) —
    /// kept as one authoritative body so tests drive the same
    /// `Activate`/`ActivateFailed` mapping the ABI callback does.
    ///
    /// **RT contract:** non-RT only — allocates per-block scratch.
    ///
    /// # Errors
    ///
    /// As [`PluginAudioProcessor::activate`]: config or preparation
    /// failure reports `ActivateFailed` (staying `Initialized`); a
    /// lifecycle rejection is a host protocol violation.
    fn activate_inner(
        shared: &'a TestGainShared,
        audio_config: &PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        // Fallible work happens while still Initialized so a failure
        // can honestly report `ActivateFailed` (legal from
        // Initialized only).
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = block::to_process_config(audio_config)
            .map_err(|_| fail("unsupported audio configuration"))?;
        let mut processor = TestGainProcessor::new();
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
        // Activation resets parameter state to the default (the
        // fixture's deterministic per-activation law): the mirror
        // reports the same value `get_value` will observe.
        shared.gain_mirror.store(GAIN_PARAM.default_value);
        Ok(Self {
            shared,
            processor,
            context,
            config,
            scratch: BlockScratch::new(block::MAX_CHANNELS, block::MAX_EVENTS_PER_BLOCK),
        })
    }
}

impl<'a> PluginAudioProcessor<'a, TestGainShared, TestGainMainThread<'a>>
    for TestGainAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &TestGainMainThread<'a>,
        shared: &'a TestGainShared,
        audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Self::activate_inner(shared, &audio_config)
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ClapProcessStatus, PluginError> {
        // Lifecycle gate — the per-block self-loop; `Process` is
        // illegal outside `Processing`, surfacing host protocol
        // violations as CLAP_PROCESS_ERROR.
        self.shared
            .lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(|_| PluginError::Message("process outside processing state"))?;

        // Frame-count bound vs the activation-negotiated maximum.
        let frames_count = audio.frames_count();
        if frames_count > self.config.max_frames() {
            return Err(PluginError::Message(
                "frames_count exceeds negotiated maximum",
            ));
        }

        // Borrowed views over the single declared port pair. A missing
        // pair is processed as an empty block (events still flush).
        let mut views = match audio.port_pair(0) {
            Some(mut pair) => {
                block::build_block_views(&mut pair, &mut self.scratch, self.config.max_frames())?
            }
            None => BlockViews::empty(),
        };
        // Caller-owned reborrow array — see `block::planar_views`.
        let mut output_list: [&mut [f32]; block::MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = block::planar_views(&mut views, &mut output_list)?;

        // Mirror sync-in, then sample-accurate event application, then
        // sync-out — keeps `params.value` honest on the main thread.
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
        let event_list =
            block::translate_events(events.input, &mut self.scratch, &GAIN_PARAM, frames_count)?;
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result =
            self.processor
                .process(&inputs, &mut outputs, event_list, None, &mut self.context);
        self.shared.gain_mirror.store(self.processor.gain_plain());
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestGainMainThread<'a>) {
        // Activated → Initialized. A deactivate while Processing is a
        // host protocol violation: the transition is rejected and the
        // machine keeps its honest state for the eventual Destroy
        // check (which will then also reject — by design).
        let _ = self.shared.lifecycle.transition(LifecycleEvent::Deactivate);
    }

    fn reset(&mut self) {
        // CLAP `reset` clears processing state but not parameter
        // values — the gain state *is* the parameter value, so reset
        // resyncs from the authoritative mirror.
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
    }

    fn start_processing(&mut self) -> Result<(), PluginError> {
        self.shared
            .lifecycle
            .transition(LifecycleEvent::StartProcessing)
            .map_err(|_| PluginError::Message("start_processing outside Activated state"))?;
        Ok(())
    }

    fn stop_processing(&mut self) {
        // No error channel — an illegal stop (e.g. never started) is a
        // recorded rejection; the machine stays honest.
        let _ = self
            .shared
            .lifecycle
            .transition(LifecycleEvent::StopProcessing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clack_common::events::Pckn;
    use clack_common::events::event_types::ParamValueEvent;
    use clack_common::events::io::EventBuffer;
    use clack_host::entry::PluginEntry;
    use clack_host::host::HostInfo;
    use clack_host::plugin::PluginInstance;
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputAudioBuffers, InputChannel,
        OutputAudioBuffers,
    };
    use clack_plugin::entry::SinglePluginEntry;

    use crate::block::MAX_CHANNELS;
    use ruviform_plugin_core::LifecycleState;

    /// Issue #22 RT-verification lane: `AllocDisabler` swaps the test
    /// binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window.
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    const FRAMES: u32 = 64;
    const FRAMES_USIZE: usize = 64;
    const RATE: u32 = 48_000;

    /// An activation config at 48 kHz bounded to `max_frames`.
    fn config(max_frames: u32) -> PluginAudioConfiguration {
        PluginAudioConfiguration {
            sample_rate: f64::from(RATE),
            min_frames_count: 1,
            max_frames_count: max_frames,
        }
    }

    /// A `ParamValue` event for our gain parameter — `value` is the
    /// *plain* multiplier (the CLAP wire domain).
    const fn gain_event(time: u32, value: f64) -> ParamValueEvent {
        ParamValueEvent::new(time, ClapId::new(GAIN_PARAM.id.0), Pckn::match_all(), value)
    }

    /// One `process` call against `ap` — the smallest complete
    /// `Process`/`Events` plumbing a host builds for `clap_process`.
    fn process_block(
        ap: &mut TestGainAudioProcessor<'_>,
        audio: Audio<'_>,
        input_buffer: &EventBuffer,
        output_buffer: &mut EventBuffer,
    ) -> Result<ClapProcessStatus, PluginError> {
        let input_events = input_buffer.as_input();
        let mut output_events = output_buffer.as_output();
        ap.process(
            Process {
                transport: None,
                steady_time: None,
            },
            audio,
            Events {
                input: &input_events,
                output: &mut output_events,
            },
        )
    }

    /// An `Audio` with no ports at all — legal CLAP input (audio may
    /// be absent); `port_pair(0)` is `None` → the empty-block path.
    fn empty_audio() -> Audio<'static> {
        InputAudioBuffers::empty().as_plugin_audio()
    }

    /// Initialized shared state plus an activated audio processor —
    /// the legal path every test starts from.
    fn activated(shared: &TestGainShared, max_frames: u32) -> TestGainAudioProcessor<'_> {
        TestGainAudioProcessor::activate_inner(shared, &config(max_frames)).unwrap()
    }

    /// A stereo `f32` port pair: distinct input/output backing `Vec`s
    /// plus the `AudioPorts` descriptor storage a host would own for
    /// one `process` call — the same fixture shape the adapter's own
    /// tests build through clack-host's safe builders.
    struct StereoFixture {
        inputs: [Vec<f32>; MAX_CHANNELS],
        outputs: [Vec<f32>; MAX_CHANNELS],
        input_ports: AudioPorts,
        output_ports: AudioPorts,
    }

    impl StereoFixture {
        fn new(frames: usize) -> Self {
            Self {
                inputs: [vec![0.5; frames], vec![-0.25; frames]],
                outputs: [vec![0.0; frames], vec![0.0; frames]],
                input_ports: AudioPorts::with_capacity(MAX_CHANNELS, 1),
                output_ports: AudioPorts::with_capacity(MAX_CHANNELS, 1),
            }
        }

        fn guards(&mut self) -> (InputAudioBuffers<'_>, OutputAudioBuffers<'_>) {
            let Self {
                inputs,
                outputs,
                input_ports,
                output_ports,
            } = self;
            let input_buffers = input_ports.with_input_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_input_only(
                    inputs.iter_mut().map(InputChannel::variable),
                ),
            }]);
            let output_buffers = output_ports.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only(
                    outputs.iter_mut().map(Vec::as_mut_slice),
                ),
            }]);
            (input_buffers, output_buffers)
        }
    }

    // -- Lifecycle mapping through TestGainClapPlugin's own objects ------

    #[test]
    fn lifecycle_happy_path_maps_created_to_destroyed() {
        let shared = TestGainShared::initialize().unwrap();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);

        let mut ap = activated(&shared, FRAMES);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);

        ap.start_processing().unwrap();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);

        // One empty-ports block — `Process` is a self-loop: the block
        // runs (SLEEP: no channels) and the machine stays Processing.
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        assert_eq!(
            process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).unwrap(),
            ClapProcessStatus::Sleep
        );
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);

        ap.stop_processing();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);

        let main = TestGainMainThread { shared: &shared };
        ap.deactivate(&main); // consumes the audio processor
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);

        shared.destroy();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Destroyed);
    }

    #[test]
    fn lifecycle_process_before_start_is_protocol_violation() {
        let shared = TestGainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);

        // `process` while Activated-but-not-started — the `Process`
        // self-loop gate rejects and the honest state is kept.
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        assert!(process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).is_err());
        assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);

        // The instance is still usable: a legal sequence succeeds.
        ap.start_processing().unwrap();
        assert!(process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).is_ok());
        ap.stop_processing();
        let main = TestGainMainThread { shared: &shared };
        ap.deactivate(&main);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);
    }

    #[test]
    fn lifecycle_activate_failure_reports_initialized() {
        let shared = TestGainShared::initialize().unwrap();
        // Zero sample rate — `to_process_config` rejects →
        // `ActivateFailed` → the machine lands back on Initialized.
        let bad = PluginAudioConfiguration {
            sample_rate: 0.0,
            min_frames_count: 1,
            max_frames_count: FRAMES,
        };
        assert!(TestGainAudioProcessor::activate_inner(&shared, &bad).is_err());
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);
    }

    // -- Real process path -------------------------------------------------

    #[test]
    fn rt_process_drives_paired_stereo_block_allocation_free() {
        // RT-contract test: the full `process` path — lifecycle gate,
        // frame bound, borrowed planar views, event translation,
        // processor, mirror sync and status map — is bounded and
        // performs no allocation (scratch is pre-allocated at
        // activate). The Issue #22 lane *observes* that:
        // `assert_no_alloc` aborts the test binary on any
        // (de)allocation inside the window. Setup — fixture/
        // `EventBuffer` allocation — stays outside.
        let shared = TestGainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(1.5);

        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let status = assert_no_alloc::assert_no_alloc(|| {
                process_block(&mut ap, audio, &input_buffer, &mut output_buffer)
            })
            .unwrap();
            assert_eq!(status, ClapProcessStatus::Continue);
        }
        // The output buffers — the host's own memory, zero copy —
        // carry input × gain (plain 1.5 from the mirror).
        assert_eq!(fixture.outputs[0], vec![0.75; FRAMES_USIZE]);
        assert_eq!(fixture.outputs[1], vec![-0.375; FRAMES_USIZE]);
        assert_eq!(shared.gain_mirror.load(), 1.5);
    }

    #[test]
    fn process_in_place_block_scales_shared_region() {
        let shared = TestGainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(1.5);

        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let io_ptrs = [
            fixture.inputs[0].as_mut_ptr(),
            fixture.inputs[1].as_mut_ptr(),
        ];
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            // In-place request: the output channel list *is* the input
            // channel list (CLAP in-place-pair semantics).
            let mut shared_list = io_ptrs;
            output_buffers.as_raw_buffers()[0].data32 = shared_list.as_mut_ptr();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            assert_eq!(
                process_block(&mut ap, audio, &input_buffer, &mut output_buffer).unwrap(),
                ClapProcessStatus::Continue
            );
        }
        assert_eq!(fixture.inputs[0], vec![0.75; FRAMES_USIZE]);
        assert_eq!(fixture.inputs[1], vec![-0.375; FRAMES_USIZE]);
    }

    #[test]
    fn process_applies_param_event_sample_accurately() {
        let shared = TestGainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(1.0);

        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let mut input_buffer = EventBuffer::new();
        // Plain 0.5 on the wire from frame 32 → normalized 0.25.
        input_buffer.push(&gain_event(32, 0.5));
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            assert_eq!(
                process_block(&mut ap, audio, &input_buffer, &mut output_buffer).unwrap(),
                ClapProcessStatus::Continue
            );
        }
        // Frames 0..32 at the block-start gain (1.0), frames 32.. at
        // the event's plain 0.5 — input left channel is 0.5 everywhere.
        assert_eq!(fixture.outputs[0][..32], [0.5; 32]);
        assert_eq!(fixture.outputs[0][32..], [0.25; 32]);
        // And the mirror carries the post-block value for params.
        assert_eq!(shared.gain_mirror.load(), 0.5);
    }

    #[test]
    fn process_without_audio_ports_flushes_events_and_sleeps() {
        let shared = TestGainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();

        // No audio buffers — the empty-block path still applies
        // parameter events (mirror update observable via params).
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&gain_event(0, 0.5));
        let mut output_buffer = EventBuffer::new();
        assert_eq!(
            process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).unwrap(),
            ClapProcessStatus::Sleep
        );
        assert_eq!(shared.gain_mirror.load(), 0.5);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);
    }

    // -- End-to-end through the real clap_plugin ABI ----------------------

    /// Instantiates the plugin through the exported `clap_entry` via
    /// clack-host — the same path a real host drives (`plugin.init`
    /// through `plugin.destroy`). `TestGainClapPlugin`'s
    /// `SinglePluginEntry` is the crate's one authoritative entry body.
    #[test]
    fn host_driven_entry_runs_full_lifecycle_through_abi() {
        let entry =
            PluginEntry::load_from_clack::<SinglePluginEntry<TestGainClapPlugin>>(c"gain.clap")
                .unwrap();
        let host = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let mut instance = PluginInstance::<()>::new(
            |()| (),
            |()| (),
            &entry,
            c"org.ruviform-test.clap-test-gain",
            &host,
        )
        .unwrap();

        // plugin.activate through the real ABI — Activate mapping.
        let stopped = instance.activate(|(), ()| (), config(FRAMES)).unwrap();
        // start_processing — StartProcessing mapping.
        let mut started = stopped.start_processing().unwrap();

        // A real paired-stereo block through clap_process.
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let input_events = input_buffer.as_input();
            let mut output_events = output_buffer.as_output();
            let status = started
                .process(
                    &input_buffers,
                    &mut output_buffers,
                    &input_events,
                    &mut output_events,
                    None,
                    None,
                )
                .unwrap();
            assert_eq!(status, ClapProcessStatus::Continue);
        }
        // Default gain is 1.0 — outputs carry the inputs verbatim.
        assert_eq!(fixture.outputs[0], vec![0.5; FRAMES_USIZE]);
        assert_eq!(fixture.outputs[1], vec![-0.25; FRAMES_USIZE]);

        // stop_processing → deactivate → drop(instance): the
        // StopProcessing → Deactivate → Destroy mappings run through
        // the real ABI (destroy fires on instance drop).
        let stopped = started.stop_processing();
        instance.deactivate(stopped);
        drop(instance);
    }

    /// The `params` extension's main-thread surface: `get_value`
    /// reports the mirrored plain gain, and `flush` folds `ParamValue`
    /// events into it — observable without a process block.
    #[test]
    fn params_extension_reports_and_flushes_gain() {
        let shared = TestGainShared::initialize().unwrap();
        let main = TestGainMainThread { shared: &shared };
        assert_eq!(main.get_value(ClapId::new(0)), Some(1.0));
        assert_eq!(main.get_value(ClapId::new(7)), None);

        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&gain_event(0, 1.5)); // plain 1.5 on the wire
        let input_events = input_buffer.as_input();
        let mut output_events = OutputEvents::void();
        main.flush(&input_events, &mut output_events);
        assert_eq!(shared.gain_mirror.load(), 1.5);
        assert_eq!(main.get_value(ClapId::new(0)), Some(1.5));
    }
}
