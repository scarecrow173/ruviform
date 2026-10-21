//! The `clack` wiring layer: plugin descriptor, factory, entry point,
//! and the thin callback shell that drives the [`adapt`] translation
//! layer — no binding logic lives here that cannot be exercised
//! through `adapt`'s testable functions.
//!
//! ## Lifecycle mapping (CLAP → neutral [`Lifecycle`])
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
//! audio processor's [`GainProcessor`] (RT path) and an `AtomicU64`
//! mirror in [`GainShared`] (`params` extension queries, `flush`).
//! Each `process` block syncs local state from the mirror, applies
//! sample-accurate events, and stores the resulting value back —
//! three atomic ops per block, all allocation-free.

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

use crate::adapt::{self, AtomicLifecycle, BlockScratch, ParamMirror};
use crate::gain::{GAIN_PARAM, GainProcessor};

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the gain parameter mirror.
#[derive(Debug)]
pub(crate) struct GainShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The gain parameter's current plain value, mirrored for the
    /// main-thread `params` extension.
    gain_mirror: ParamMirror,
}

impl PluginShared<'_> for GainShared {}

impl Drop for GainShared {
    fn drop(&mut self) {
        // `plugin.destroy` — legal from Created/Initialized only; a
        // host destroying an activated/processing instance is a
        // protocol violation: the transition is *rejected* (the
        // machine does not silently fabricate a Deactivate), and the
        // instance then dies with the honest state recorded.
        let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port and
/// parameter metadata live here.
pub(crate) struct GainMainThread<'a> {
    shared: &'a GainShared,
}

impl<'a> PluginMainThread<'a, GainShared> for GainMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`GainProcessor`], its context and all per-block scratch.
pub(crate) struct GainAudioProcessor<'a> {
    shared: &'a GainShared,
    processor: GainProcessor,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `PoC` plugin type — a unit struct tying the associated types
/// together, exported through `SinglePluginEntry`.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct GainClapPlugin;

impl Plugin for GainClapPlugin {
    type AudioProcessor<'a> = GainAudioProcessor<'a>;
    type Shared<'a> = GainShared;
    type MainThread<'a> = GainMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&GainShared>) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for GainClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform.clap-gain-poc", "Ruviform Gain PoC")
            .with_vendor("Ruviform")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_description("CLAP binding-method PoC — minimal gain plugin")
            .with_features([features::AUDIO_EFFECT, features::STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<GainShared, PluginError> {
        // `plugin.init` — the CLAP init hook runs the shared/main
        // constructors; Created → Initialize is unconditional for this
        // stateless init.
        let shared = GainShared {
            lifecycle: AtomicLifecycle::new(),
            gain_mirror: ParamMirror::new(GAIN_PARAM.default_value),
        };
        shared
            .lifecycle
            .transition(LifecycleEvent::Initialize)
            .map_err(|_| PluginError::Message("init outside Created state"))?;
        Ok(shared)
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a GainShared,
    ) -> Result<GainMainThread<'a>, PluginError> {
        // Nothing fallible here — but the failure arm documents the
        // `InitFailed` mapping for future fallible init work.
        Ok(GainMainThread { shared })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for GainMainThread<'_> {
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

impl PluginMainThreadParams for GainMainThread<'_> {
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
        Some(value.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Main-thread flush (e.g. changes applied while inactive):
        // fold every ParamValue for our param into the mirror; offsets
        // carry no meaning outside a process block.
        for event in input_parameter_changes.iter() {
            if let Some(neutral) = adapt::translate_param_event(event, &GAIN_PARAM) {
                if let EventKind::ParamValue(param_event) = neutral.kind() {
                    self.shared
                        .gain_mirror
                        .store(GAIN_PARAM.denormalize(param_event.value));
                }
            }
        }
    }
}

impl PluginAudioProcessorParams for GainAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Audio-thread flush: mirror the change and resync the live
        // processor — CLAP does not guarantee offsets here.
        for event in input_parameter_changes.iter() {
            if let Some(neutral) = adapt::translate_param_event(event, &GAIN_PARAM) {
                if let EventKind::ParamValue(param_event) = neutral.kind() {
                    self.shared
                        .gain_mirror
                        .store(GAIN_PARAM.denormalize(param_event.value));
                }
            }
        }
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
    }
}

// ---------------------------------------------------------------------------
// Audio processor
// ---------------------------------------------------------------------------

impl<'a> PluginAudioProcessor<'a, GainShared, GainMainThread<'a>> for GainAudioProcessor<'a> {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &GainMainThread<'a>,
        shared: &'a GainShared,
        audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        // Fallible work happens while still Initialized so a failure
        // can honestly report `ActivateFailed` (legal from
        // Initialized only).
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = adapt::to_process_config(&audio_config)
            .map_err(|_| fail("unsupported audio configuration"))?;
        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
        // Params survive activation — seed the processor from the
        // authoritative mirror rather than re-defaulting.
        processor.set_gain_plain(shared.gain_mirror.load());
        Ok(Self {
            shared,
            processor,
            context,
            config,
            scratch: BlockScratch::new(adapt::MAX_CHANNELS, adapt::MAX_EVENTS_PER_BLOCK),
        })
    }

    fn process(
        &mut self,
        process: Process,
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
            Some(mut pair) => adapt::build_block_views(&mut pair, &mut self.scratch)?,
            None => adapt::BlockViews::empty(),
        };
        // Caller-owned reborrow array — see `adapt::planar_views`.
        let mut output_list: [&mut [f32]; adapt::MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = adapt::planar_views(&mut views, &mut output_list)?;

        // Mirror sync-in, then sample-accurate event application, then
        // sync-out — keeps `params.value` honest on the main thread.
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
        let event_list = adapt::translate_events(
            events.input,
            self.scratch.events_scratch(),
            &GAIN_PARAM,
            frames_count,
        )?;
        let transport = process
            .transport
            .map(|t| adapt::adapt_transport(t, self.config.sample_rate(), process.steady_time));
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
        adapt::map_process_result(result)
    }

    fn deactivate(self, _main_thread: &GainMainThread<'a>) {
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
