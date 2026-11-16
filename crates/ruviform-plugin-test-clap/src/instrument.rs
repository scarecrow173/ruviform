//! The `test_instrument` CLAP fixture — a deterministic,
//! allocation-free polyphonic test voice plus a transport-clock
//! click oracle (Issue #630 `test_instrument` slice).
//!
//! ## Voice law
//!
//! Identical to the VST3 `test_instrument` fixture's law — up to
//! [`MAX_VOICES`] voices render a triangle wave at
//! `440 Hz * 2^((key - 69) / 12)`; phase is a `u64` Q32.32
//! accumulator; `NoteOn` (velocity `0` follows the MIDI 1.0
//! running-status convention and behaves as note-off) retriggers the
//! same-key voice, else takes the first inactive voice, else steals
//! the oldest. `NoteOff` starts a fixed [`RELEASE_FRAMES`]-frame
//! linear release. `ControlChange` 7 sets master volume and CC 1
//! sets tremolo depth (canonical `u32` scale-to-max); every other
//! controller and pitch-bend/pressure/program event is a defined
//! no-op. Channel, port and note-id targeting are ignored — the
//! fixture is channel-agnostic.
//!
//! ## Mix law
//!
//! `out[c][n] = voice * gain * volume * tremolo + click[n]` where
//! `voice = Σ triangle(phase_v) * velocity_v * envelope_v` — a 4 Hz
//! triangle LFO scaled by the CC 1 depth, written identically to
//! every output channel (mono instrument → dual mono).
//!
//! ## Transport click law
//!
//! While the CLAP `Process::transport` snapshot reports playing
//! **and** carries beats timeline + tempo, the fixture emits a
//! single-sample impulse of amplitude `CLICK_AMP * volume` at every
//! `ppqn`-tick boundary strictly inside the block. CLAP transport
//! carries no PPQN, so this fixture fixes [`CLAP_PPQN`] = 960 —
//! `position_ticks = round(song_pos_beats * 960)`,
//! `bpm_milli = round(tempo * 1000)`. Tick→frame conversion is the
//! same exact `i128` ceiling division the VST3 fixture uses; each
//! block derives clicks from its own snapshot, so loop/seek/chase
//! are exercised purely through delivered transport state.
//!
//! The DSP, voice law and click law are this fixture's own
//! (independent of every production adapter); only the `clack-*`
//! binding layer and the neutral contracts are shared.

use core::fmt::Write as _;
use std::ffi::CStr;

use clack_common::events::io::{InputEvents, OutputEvents};
use clack_common::plugin::features;
use clack_common::utils::{ClapId, Cookie};
use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_extensions::note_ports::{
    NoteDialect, NoteDialects, NotePortInfo, NotePortInfoWriter, PluginNotePorts,
    PluginNotePortsImpl,
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
use crate::gain::GAIN_PARAM;
use crate::instrument_core::TestInstrumentCore;

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the gain parameter mirror —
/// the same shape `test_gain` uses.
pub(crate) struct TestInstrumentShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The gain parameter's current plain value, mirrored for the
    /// main-thread `params` extension.
    gain_mirror: ParamMirror,
}

impl PluginShared<'_> for TestInstrumentShared {}

impl TestInstrumentShared {
    /// The `plugin.init` hook's fallible core.
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

    /// The `plugin.destroy` transition — invoked from [`Drop`].
    fn destroy(&self) {
        let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
    }
}

impl Drop for TestInstrumentShared {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port and
/// parameter metadata live here.
pub(crate) struct TestInstrumentMainThread<'a> {
    shared: &'a TestInstrumentShared,
}

impl<'a> PluginMainThread<'a, TestInstrumentShared> for TestInstrumentMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`TestInstrumentCore`], its context and all per-block scratch.
pub(crate) struct TestInstrumentAudioProcessor<'a> {
    shared: &'a TestInstrumentShared,
    processor: TestInstrumentCore,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `test_instrument` plugin type — a unit struct tying the
/// associated types together.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TestInstrumentClapPlugin;

impl Plugin for TestInstrumentClapPlugin {
    type AudioProcessor<'a> = TestInstrumentAudioProcessor<'a>;
    type Shared<'a> = TestInstrumentShared;
    type MainThread<'a> = TestInstrumentMainThread<'a>;

    fn declare_extensions(
        builder: &mut PluginExtensions<Self>,
        _shared: Option<&TestInstrumentShared>,
    ) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginNotePorts>()
            .register::<PluginParams>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for TestInstrumentClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform-test.clap-test-instrument", "test_instrument")
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — deterministic instrument for host verification",
            )
            .with_features([
                features::INSTRUMENT,
                features::SYNTHESIZER,
                features::STEREO,
            ])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<TestInstrumentShared, PluginError> {
        TestInstrumentShared::initialize()
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a TestInstrumentShared,
    ) -> Result<TestInstrumentMainThread<'a>, PluginError> {
        Ok(TestInstrumentMainThread { shared })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for TestInstrumentMainThread<'_> {
    fn count(&self, _is_input: bool) -> u32 {
        // One stereo port pair, same shape as `test_gain`: the
        // instrument is generative and ignores its input, but the
        // pair supplies the output buffers the block layer needs.
        1
    }

    fn get(&self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index != 0 {
            return;
        }
        writer.set(&AudioPortInfo {
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
// Note ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginNotePortsImpl for TestInstrumentMainThread<'_> {
    fn count(&self, is_input: bool) -> u32 {
        // One MIDI note input, no note output — the instrument is
        // generative and emits no note data.
        u32::from(is_input)
    }

    fn get(&self, index: u32, is_input: bool, writer: &mut NotePortInfoWriter) {
        if index != 0 || !is_input {
            return;
        }
        writer.set(&NotePortInfo {
            id: ClapId::new(0),
            name: b"Notes",
            supported_dialects: NoteDialects::CLAP | NoteDialects::MIDI,
            preferred_dialect: Some(NoteDialect::Midi),
        });
    }
}

// ---------------------------------------------------------------------------
// Params extension
// ---------------------------------------------------------------------------

impl PluginMainThreadParams for TestInstrumentMainThread<'_> {
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
            module: b"",
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
        value
            .is_finite()
            .then(|| value.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
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

impl PluginAudioProcessorParams for TestInstrumentAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
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

impl<'a> TestInstrumentAudioProcessor<'a> {
    /// The body of [`PluginAudioProcessor::activate`], minus the host
    /// handles — the same mapping `test_gain` documents.
    ///
    /// **RT contract:** non-RT only — allocates per-block scratch.
    ///
    /// # Errors
    ///
    /// As [`PluginAudioProcessor::activate`]: config or preparation
    /// failure reports `ActivateFailed`; a lifecycle rejection is a
    /// host protocol violation.
    fn activate_inner(
        shared: &'a TestInstrumentShared,
        audio_config: &PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = block::to_process_config(audio_config)
            .map_err(|_| fail("unsupported audio configuration"))?;
        let mut processor = TestInstrumentCore::new();
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
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

impl<'a> PluginAudioProcessor<'a, TestInstrumentShared, TestInstrumentMainThread<'a>>
    for TestInstrumentAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &TestInstrumentMainThread<'a>,
        shared: &'a TestInstrumentShared,
        audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Self::activate_inner(shared, &audio_config)
    }

    fn process(
        &mut self,
        process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ClapProcessStatus, PluginError> {
        // Lifecycle gate — the per-block self-loop; `Process` is
        // illegal outside `Processing`.
        self.shared
            .lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(|_| PluginError::Message("process outside processing state"))?;

        let frames_count = audio.frames_count();
        if frames_count > self.config.max_frames() {
            return Err(PluginError::Message(
                "frames_count exceeds negotiated maximum",
            ));
        }

        let mut views = match audio.port_pair(0) {
            Some(mut pair) => {
                block::build_block_views(&mut pair, &mut self.scratch, self.config.max_frames())?
            }
            None => BlockViews::empty(),
        };
        let mut output_list: [&mut [f32]; block::MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = block::planar_views(&mut views, &mut output_list)?;

        // Mirror sync-in, then sample-accurate event application, then
        // sync-out — the same three-atomic-ops law `test_gain` uses.
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
        let event_list =
            block::translate_events(events.input, &mut self.scratch, frames_count, |unknown| {
                block::translate_instrument_event(unknown, &GAIN_PARAM)
            })?;
        let transport = block::transport_info(process.transport, self.config.sample_rate().get());
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result = self.processor.process(
            &inputs,
            &mut outputs,
            event_list,
            transport,
            &mut self.context,
        );
        self.shared.gain_mirror.store(self.processor.gain_plain());
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestInstrumentMainThread<'a>) {
        let _ = self.shared.lifecycle.transition(LifecycleEvent::Deactivate);
    }

    fn reset(&mut self) {
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
        let _ = self
            .shared
            .lifecycle
            .transition(LifecycleEvent::StopProcessing);
    }
}
