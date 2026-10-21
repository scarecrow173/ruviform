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
use clack_extensions::thread_check::HostThreadCheck;
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
pub(crate) struct GainShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The gain parameter's current plain value, mirrored for the
    /// main-thread `params` extension.
    gain_mirror: ParamMirror,
    /// Thread-safe host handle kept for `process`'s thread-contract
    /// check — `None` for hostless test construction.
    host: Option<HostSharedHandle<'static>>,
    /// The host's `thread-check` extension token, resolved once at
    /// `plugin.init` — `None` when the host does not provide it.
    thread_check: Option<HostThreadCheck>,
}

impl PluginShared<'_> for GainShared {}

impl GainShared {
    /// The `plugin.init` hook's fallible core, minus the
    /// [`HostSharedHandle`] the factory signature carries: Created →
    /// Initialize is unconditional for this stateless init. Kept as
    /// one authoritative body so tests drive the same `Initialize`
    /// mapping the ABI callback does. `initialize_with_host` threads
    /// the host's `thread-check` extension through.
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
            host: None,
            thread_check: None,
        };
        shared
            .lifecycle
            .transition(LifecycleEvent::Initialize)
            .map_err(|_| PluginError::Message("init outside Created state"))?;
        Ok(shared)
    }

    /// `initialize` plus the host wiring: resolves `thread-check`
    /// once at `plugin.init` so `process`'s per-block cost stays a
    /// single `is_audio_thread` FFI call (no extension lookup on the
    /// RT path).
    // SAFETY: `with_arbitrary_lifetime` widens the host handle to
    // `'static`; the `clap_host` pointer it wraps stays valid for the
    // whole plugin instance lifetime per the CLAP contract (the host
    // owns the instance from `plugin.init` through `plugin.destroy`),
    // and the stored handle is only dereferenced while the instance
    // is alive.
    #[expect(unsafe_code)]
    fn initialize_with_host(host: HostSharedHandle<'_>) -> Result<Self, PluginError> {
        let mut shared = Self::initialize()?;
        shared.thread_check = host.get_extension::<HostThreadCheck>();
        shared.host = Some(unsafe { host.with_arbitrary_lifetime() });
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

impl Drop for GainShared {
    fn drop(&mut self) {
        self.destroy();
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

    fn new_shared(host: HostSharedHandle<'_>) -> Result<GainShared, PluginError> {
        // `plugin.init` — the CLAP init hook runs the shared/main
        // constructors.
        GainShared::initialize_with_host(host)
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
        // `clamp` propagates NaN — a non-finite parse is rejected,
        // never handed to the host as a value.
        value
            .is_finite()
            .then(|| value.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Main-thread flush (e.g. changes applied while inactive):
        // fold every ParamValue for our param into the mirror; offsets
        // carry no meaning outside a process block. The scan is
        // bounded like the process path's.
        for event in input_parameter_changes
            .iter()
            .take(adapt::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(neutral) = adapt::translate_param_event(event, &GAIN_PARAM)
                && let EventKind::ParamValue(param_event) = neutral.kind()
            {
                self.shared
                    .gain_mirror
                    .store(GAIN_PARAM.denormalize(param_event.value));
            }
        }
    }
}

impl PluginAudioProcessorParams for GainAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Audio-thread flush: mirror the change and resync the live
        // processor — CLAP does not guarantee offsets here. The scan
        // is bounded like the process path's.
        for event in input_parameter_changes
            .iter()
            .take(adapt::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(neutral) = adapt::translate_param_event(event, &GAIN_PARAM)
                && let EventKind::ParamValue(param_event) = neutral.kind()
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

impl<'a> GainAudioProcessor<'a> {
    /// The body of [`PluginAudioProcessor::activate`], minus the host
    /// handles the trait signature carries (unused by this `PoC`) —
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
        shared: &'a GainShared,
        audio_config: &PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        // Fallible work happens while still Initialized so a failure
        // can honestly report `ActivateFailed` (legal from
        // Initialized only).
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = adapt::to_process_config(audio_config)
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
}

impl<'a> PluginAudioProcessor<'a, GainShared, GainMainThread<'a>> for GainAudioProcessor<'a> {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &GainMainThread<'a>,
        shared: &'a GainShared,
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
        // Thread-contract check (CLAP `thread-check`): when the host
        // provides the extension and declares this is *not* the audio
        // thread, `process` is a host protocol violation — a defined
        // rejection, never UB. `None` (host without the extension)
        // degrades gracefully: no check, unchanged behavior.
        if let (Some(thread_check), Some(host)) = (self.shared.thread_check, self.shared.host)
            && thread_check.is_audio_thread(&host) == Some(false)
        {
            return Err(PluginError::Message("process called off the audio thread"));
        }

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
                adapt::build_block_views(&mut pair, &mut self.scratch, self.config.max_frames())?
            }
            None => adapt::BlockViews::empty(),
        };
        // Caller-owned reborrow array — see `adapt::planar_views`.
        let mut output_list: [&mut [f32]; adapt::MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = adapt::planar_views(&mut views, &mut output_list)?;

        // Mirror sync-in, then sample-accurate event application, then
        // sync-out — keeps `params.value` honest on the main thread.
        self.processor
            .set_gain_plain(self.shared.gain_mirror.load());
        let event_list =
            adapt::translate_events(events.input, &mut self.scratch, &GAIN_PARAM, frames_count)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use clack_common::events::Pckn;
    use clack_common::events::event_types::ParamValueEvent;
    use clack_common::events::event_types::{TransportEvent, TransportFlags};
    use clack_common::events::io::EventBuffer;
    use clack_common::events::{EventFlags, EventHeader};
    use clack_common::utils::{BeatTime, SecondsTime};
    use clack_host::entry::PluginEntry;
    use clack_host::host::HostInfo;
    use clack_host::plugin::{PluginInstance, PluginInstanceError};
    use clack_host::process::audio_buffers::InputAudioBuffers;
    use clack_plugin::entry::SinglePluginEntry;

    use crate::adapt::test_support::StereoFixture;
    use clack_extensions::thread_check::{HostThreadCheck, HostThreadCheckImpl};
    use clack_host::host::{HostExtensions, HostHandlers, SharedHandler};
    use ruviform_plugin_core::LifecycleState;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Issue #22 RT-verification lane: `AllocDisabler` swaps the test
    /// binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window. One static per
    /// crate test binary; `gain.rs` tests measure against the same
    /// allocator.
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

    /// One `process` call against `ap` with no transport — the
    /// smallest complete `Process`/`Events` plumbing a host builds
    /// for `clap_process`.
    fn process_block(
        ap: &mut GainAudioProcessor<'_>,
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
    fn activated(shared: &GainShared, max_frames: u32) -> GainAudioProcessor<'_> {
        GainAudioProcessor::activate_inner(shared, &config(max_frames)).unwrap()
    }

    // -- Lifecycle mapping through GainClapPlugin's own objects --------------

    #[test]
    fn lifecycle_happy_path_maps_created_to_destroyed() {
        let shared = GainShared::initialize().unwrap();
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

        let main = GainMainThread { shared: &shared };
        ap.deactivate(&main); // consumes the audio processor
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);

        // `plugin.destroy` is `GainShared::drop` — the transition body
        // is invoked explicitly so the terminal state is observable
        // (the scope-end Drop run afterwards just records a rejected
        // second Destroy — `_`).
        shared.destroy();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Destroyed);
    }

    #[test]
    fn lifecycle_reactivation_cycles_are_legal() {
        let shared = GainShared::initialize().unwrap();
        let main = GainMainThread { shared: &shared };

        for _ in 0..2 {
            let mut ap = activated(&shared, FRAMES);
            assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);
            ap.start_processing().unwrap();
            ap.stop_processing();
            ap.deactivate(&main);
            assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);
        }
    }

    #[test]
    fn lifecycle_activate_failure_reports_initialized() {
        let shared = GainShared::initialize().unwrap();
        // Zero/invalid sample rate — `to_process_config` rejects →
        // `ActivateFailed` → the machine lands back on Initialized.
        let bad = PluginAudioConfiguration {
            sample_rate: 0.0,
            min_frames_count: 1,
            max_frames_count: FRAMES,
        };
        assert!(GainAudioProcessor::activate_inner(&shared, &bad).is_err());
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);
    }

    #[test]
    fn lifecycle_process_before_start_is_protocol_violation() {
        let shared = GainShared::initialize().unwrap();
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
        let main = GainMainThread { shared: &shared };
        ap.deactivate(&main);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);
    }

    #[test]
    fn lifecycle_deactivate_while_processing_is_rejected_honestly() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();

        let main = GainMainThread { shared: &shared };
        ap.deactivate(&main); // consumes ap — protocol violation recorded
        // `deactivate` has no error channel: the rejected transition
        // is recorded and the machine stays honestly in Processing
        // (a later Destroy is likewise rejected — fail-closed).
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);
    }

    #[test]
    fn lifecycle_destroy_from_initialized_only() {
        // Activated/processing instances reject Destroy (the drop
        // records the rejection); a clean sequence reaches Destroyed.
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        ap.stop_processing();
        let main = GainMainThread { shared: &shared };
        ap.deactivate(&main);
        shared.destroy();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Destroyed);
    }

    // -- Real process path -----------------------------------------------------

    #[test]
    fn rt_process_drives_paired_stereo_block_allocation_free() {
        // RT-contract test: the full `process` path — thread-check
        // gate, lifecycle gate, frame bound, borrowed planar views,
        // event translation, transport, processor, mirror, status
        // map — is bounded and performs no allocation (scratch is
        // pre-allocated at activate; every collection is a fixed-size
        // array or a capacity-reserved Vec that never grows past
        // MAX_*). The Issue #22 lane *observes* that: `assert_no_alloc`
        // aborts the test binary on any (de)allocation inside the
        // window. Setup — fixture/`EventBuffer` allocation — stays
        // outside.
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(0.5);

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
        // carry input × gain.
        assert_eq!(fixture.outputs[0], vec![0.25; FRAMES_USIZE]);
        assert_eq!(fixture.outputs[1], vec![-0.125; FRAMES_USIZE]);
        assert_eq!(shared.gain_mirror.load(), 0.5);
    }

    #[test]
    fn process_in_place_block_scales_shared_region() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(0.5);

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
        assert_eq!(fixture.inputs[0], vec![0.25; FRAMES_USIZE]);
        assert_eq!(fixture.inputs[1], vec![-0.125; FRAMES_USIZE]);
    }

    #[test]
    fn process_rejects_frames_beyond_negotiated_max() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, 32); // negotiated max 32 frames
        ap.start_processing().unwrap();

        // Host delivers a 64-frame block anyway — defined rejection.
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            assert!(process_block(&mut ap, audio, &input_buffer, &mut output_buffer).is_err());
        }
        // The `Process` self-loop already ran — the block is a
        // processing-state rejection, not a fabricated transition.
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);
    }

    #[test]
    fn process_applies_param_event_sample_accurately() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();
        shared.gain_mirror.store(0.5);

        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&gain_event(32, 1.0)); // gain 1.0 from frame 32
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            assert_eq!(
                process_block(&mut ap, audio, &input_buffer, &mut output_buffer).unwrap(),
                ClapProcessStatus::Continue
            );
        }
        // Frames 0..32 at the block-start gain (0.5), frames 32.. at
        // the event's 1.0 — input left channel is 0.5 everywhere.
        assert_eq!(fixture.outputs[0][..32], [0.25; 32]);
        assert_eq!(fixture.outputs[0][32..], [0.5; 32]);
        // And the mirror carries the post-block value for params.
        assert_eq!(shared.gain_mirror.load(), 1.0);
    }

    #[test]
    fn process_without_audio_ports_flushes_events_and_sleeps() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();

        // No audio buffers — the empty-block path still applies
        // parameter events (mirror update observable via params).
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&gain_event(0, 0.25));
        let mut output_buffer = EventBuffer::new();
        assert_eq!(
            process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).unwrap(),
            ClapProcessStatus::Sleep
        );
        assert_eq!(shared.gain_mirror.load(), 0.25);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);
    }

    #[test]
    fn process_accepts_transport_and_steady_time() {
        let shared = GainShared::initialize().unwrap();
        let mut ap = activated(&shared, FRAMES);
        ap.start_processing().unwrap();

        // A transport event in `Process::transport` — mapped through
        // `adapt_transport`; GainProcessor ignores it, the path must
        // still run clean.
        let transport = TransportEvent {
            header: EventHeader::<TransportEvent>::new_core(0, EventFlags::empty()),
            flags: TransportFlags::IS_PLAYING | TransportFlags::HAS_SECONDS_TIMELINE,
            song_pos_beats: BeatTime::from_int(0),
            song_pos_seconds: SecondsTime::from_float(1.0),
            tempo: 120.0,
            tempo_inc: 0.0,
            loop_start_beats: BeatTime::from_int(0),
            loop_end_beats: BeatTime::from_int(0),
            loop_start_seconds: SecondsTime::from_float(0.0),
            loop_end_seconds: SecondsTime::from_float(2.0),
            bar_start: BeatTime::from_int(0),
            bar_number: 0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        };
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            let input_events = input_buffer.as_input();
            let mut output_events = output_buffer.as_output();
            assert_eq!(
                ap.process(
                    Process {
                        transport: Some(&transport),
                        steady_time: Some(1_234),
                    },
                    audio,
                    Events {
                        input: &input_events,
                        output: &mut output_events,
                    },
                )
                .unwrap(),
                ClapProcessStatus::Continue
            );
        }
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);
    }

    // -- End-to-end through the real clap_plugin ABI ----------------------------

    /// Instantiates the plugin through the exported `clap_entry` via
    /// clack-host — the same path a real host drives (`plugin.init`
    /// through `plugin.destroy`). `GainClapPlugin`'s `SinglePluginEntry`
    /// is the crate's one authoritative entry body.
    fn host_instance() -> (PluginEntry, PluginInstance<()>) {
        let entry = PluginEntry::load_from_clack::<SinglePluginEntry<GainClapPlugin>>(c"gain.clap")
            .unwrap();
        let host = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let instance = PluginInstance::<()>::new(
            |()| (),
            |()| (),
            &entry,
            c"org.ruviform.clap-gain-poc",
            &host,
        )
        .unwrap();
        (entry, instance)
    }

    // -- thread-check host -------------------------------------------------

    /// A test host that registers the CLAP `thread-check` extension
    /// and answers `is_audio_thread`/`is_main_thread` from flippable
    /// atomics — both the accept and the defined-rejection path are
    /// drivable through the real ABI.
    struct RtCheckHost;

    /// `RtCheckHost`'s `[thread-safe]` shared handler; the atomics are
    /// cloned in from the test so the answer can flip mid-scenario.
    struct RtCheckShared {
        is_audio: Arc<AtomicBool>,
        is_main: Arc<AtomicBool>,
    }

    impl SharedHandler<'_> for RtCheckShared {
        fn request_restart(&self) {}
        fn request_process(&self) {}
        fn request_callback(&self) {}
    }

    impl HostThreadCheckImpl for RtCheckShared {
        fn is_main_thread(&self) -> bool {
            self.is_main.load(Ordering::Relaxed)
        }
        fn is_audio_thread(&self) -> bool {
            self.is_audio.load(Ordering::Relaxed)
        }
    }

    impl HostHandlers for RtCheckHost {
        type Shared<'a> = RtCheckShared;
        type MainThread<'a> = ();
        type AudioProcessor<'a> = ();

        fn declare_extensions(builder: &mut HostExtensions<Self>, _shared: &RtCheckShared) {
            builder.register::<HostThreadCheck>();
        }
    }

    /// Like [`host_instance`], but the host answers the thread-check
    /// extension from the returned atomics.
    fn rt_check_host_instance() -> (
        PluginEntry,
        PluginInstance<RtCheckHost>,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
    ) {
        let entry = PluginEntry::load_from_clack::<SinglePluginEntry<GainClapPlugin>>(c"gain.clap")
            .unwrap();
        let host = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let is_audio = Arc::new(AtomicBool::new(true));
        let is_main = Arc::new(AtomicBool::new(true));
        let (audio_flag, main_flag) = (Arc::clone(&is_audio), Arc::clone(&is_main));
        let instance = PluginInstance::<RtCheckHost>::new(
            move |()| RtCheckShared {
                is_audio: audio_flag,
                is_main: main_flag,
            },
            |_| (),
            &entry,
            c"org.ruviform.clap-gain-poc",
            &host,
        )
        .unwrap();
        (entry, instance, is_audio, is_main)
    }

    #[test]
    fn host_driven_entry_runs_full_lifecycle_through_abi() {
        let (_entry, mut instance) = host_instance();
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

    #[test]
    fn host_driven_process_rejection_surfaces_as_error() {
        let (_entry, mut instance) = host_instance();
        let stopped = instance.activate(|(), ()| (), config(32)).unwrap();
        let mut started = stopped.start_processing().unwrap();

        // Host delivers a block beyond the negotiated 32-frame max —
        // the adapter's boundary rejection comes back through the ABI
        // as a processing error, never as UB.
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let input_events = input_buffer.as_input();
            let mut output_events = output_buffer.as_output();
            let err = started
                .process(
                    &input_buffers,
                    &mut output_buffers,
                    &input_events,
                    &mut output_events,
                    None,
                    None,
                )
                .unwrap_err();
            assert!(matches!(err, PluginInstanceError::ProcessingFailed));
        }

        let stopped = started.stop_processing();
        instance.deactivate(stopped);
        drop(instance);
    }

    // -- Thread-contract verification (Issue #22) ---------------------------

    /// `process` on a host that declares the calling thread is *not*
    /// the audio thread → defined rejection through the real ABI.
    /// The `is_audio` flag flips *after* activation to model a host
    /// that answers `is_audio_thread` per-call.
    #[test]
    fn rt_process_off_audio_thread_is_defined_rejection() {
        let (_entry, mut instance, is_audio, _is_main) = rt_check_host_instance();
        let stopped = instance.activate(|_, ()| (), config(FRAMES)).unwrap();
        let mut started = stopped.start_processing().unwrap();
        is_audio.store(false, Ordering::Relaxed);

        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let input_events = input_buffer.as_input();
            let mut output_events = output_buffer.as_output();
            let err = started
                .process(
                    &input_buffers,
                    &mut output_buffers,
                    &input_events,
                    &mut output_events,
                    None,
                    None,
                )
                .unwrap_err();
            assert!(matches!(err, PluginInstanceError::ProcessingFailed));
        }
    }

    /// The same host answering `is_audio_thread = true` keeps the
    /// normal path — the check is transparent when the contract holds.
    #[test]
    fn rt_process_on_audio_thread_passes_thread_check() {
        let (_entry, mut instance, _is_audio, _is_main) = rt_check_host_instance();
        let stopped = instance.activate(|_, ()| (), config(FRAMES)).unwrap();
        let mut started = stopped.start_processing().unwrap();

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
    }
}
