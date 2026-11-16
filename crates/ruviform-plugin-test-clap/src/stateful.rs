//! The `test_stateful` CLAP fixture — the suite's real-state plugin
//! (Issue #630 `test_stateful` slice).
//!
//! `test_stateful` carries the same port shape, parameter contract
//! and DSP law as `test_gain` (`out[c][n] = in[c][n] * gain(n)`),
//! but adds the `clap.ext.state` extension so the production host
//! bridge's `save_state`/`restore_state` verbs exercise a **real**
//! plugin-defined byte stream instead of the stateless fixtures'
//! `Unsupported` decline.
//!
//! ## State law
//!
//! The persistent state is the gain parameter's plain value, held in
//! the shared [`ParamMirror`] (an atomic `f64` cell) — the same cell
//! the `params` extension reports and each `process` block syncs
//! through. `clap.ext.state` `save` serializes it as one
//! plugin-defined byte stream:
//!
//! ```text
//! [u32 LE magic "RFST"][u32 LE version = 1][f64 LE plain gain]
//! ```
//!
//! — 16 bytes, no adapter framing (CLAP state is a single stream,
//! unlike the VST3 component/controller pair). `load` accepts exactly
//! that length: wrong length, magic, version or a non-finite value
//! is a defined `PluginError` rejection; a finite out-of-range value
//! clamps to the `ParamInfo` plain range, matching
//! `text_to_value`'s law. A `load` that changes the mirrored value
//! requests `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)` —
//! the CLAP-required announcement for parameter changes made outside
//! param events (the main thread stores the host handle for exactly
//! this call).
//!
//! Unlike `test_gain` — whose activation deterministically resets
//! the parameter to its default — `test_stateful`'s activation
//! **preserves** the mirror: a restored (or `load`-written) gain
//! survives `activate`, which is what makes save → mutate → restore
//! → save meaningful across host re-activation.
//!
//! State verbs run on the CLAP main thread and are non-RT: they may
//! allocate and do bounded I/O through the host-provided streams.
//! The RT `process` path is untouched — it still performs only the
//! three atomic mirror ops per block and never allocates.

use core::fmt::Write as _;
use std::ffi::CStr;
use std::io::{Read, Write};

use clack_common::events::io::{InputEvents, OutputEvents};
use clack_common::plugin::features;
use clack_common::stream::{InputStream, OutputStream};
use clack_common::utils::{ClapId, Cookie};
use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_extensions::params::{
    HostParams, ParamDisplayWriter, ParamInfoFlags, ParamInfoWriter, ParamRescanFlags,
    PluginAudioProcessorParams, PluginMainThreadParams, PluginParams,
};
use clack_extensions::state::{PluginState, PluginStateImpl};
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
// State blob codec
// ---------------------------------------------------------------------------

/// The state blob's magic — `"RFST"` little-endian, identifying the
/// fixture's state format to `load`'s validator.
const STATE_MAGIC: u32 = u32::from_le_bytes(*b"RFST");

/// The state blob's format version — `load` rejects anything else.
const STATE_VERSION: u32 = 1;

/// The state blob's exact length: magic + version + one `f64`.
pub(crate) const STATE_BLOB_LEN: usize = 16;

/// Serializes the plain gain into the fixture's 16-byte state blob —
/// deterministic and byte-exact for a given value.
fn encode_state_blob(plain: f64) -> [u8; STATE_BLOB_LEN] {
    let mut blob = [0u8; STATE_BLOB_LEN];
    blob[0..4].copy_from_slice(&STATE_MAGIC.to_le_bytes());
    blob[4..8].copy_from_slice(&STATE_VERSION.to_le_bytes());
    blob[8..16].copy_from_slice(&plain.to_le_bytes());
    blob
}

/// Decodes and validates a state blob — `None` on any contract
/// violation (wrong length, magic, version, non-finite value).
/// Finite out-of-range values clamp to the parameter's plain range,
/// the same law `text_to_value` applies.
fn decode_state_blob(blob: &[u8]) -> Option<f64> {
    if blob.len() != STATE_BLOB_LEN {
        return None;
    }
    if u32::from_le_bytes(blob[0..4].try_into().ok()?) != STATE_MAGIC {
        return None;
    }
    if u32::from_le_bytes(blob[4..8].try_into().ok()?) != STATE_VERSION {
        return None;
    }
    let plain = f64::from_le_bytes(blob[8..16].try_into().ok()?);
    plain
        .is_finite()
        .then(|| plain.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value))
}

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the gain parameter mirror — the
/// mirror doubles as the persisted state cell.
pub(crate) struct TestStatefulShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The gain parameter's current plain value, mirrored for the
    /// main-thread `params`/`state` extensions and synced through
    /// each `process` block.
    gain_mirror: ParamMirror,
}

impl PluginShared<'_> for TestStatefulShared {}

impl TestStatefulShared {
    /// The `plugin.init` hook's fallible core: Created → Initialize
    /// is unconditional; the mirror starts at the parameter default.
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

impl Drop for TestStatefulShared {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port,
/// parameter metadata and the `clap.ext.state` surface live here.
pub(crate) struct TestStatefulMainThread<'a> {
    shared: &'a TestStatefulShared,
    /// The host handle `state.load` needs to request a params rescan
    /// when a restored blob changes parameter values — `Some` on the
    /// real factory path, `None` only in unit tests that construct
    /// the main thread directly.
    host: Option<HostMainThreadHandle<'a>>,
}

impl<'a> PluginMainThread<'a, TestStatefulShared> for TestStatefulMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`TestGainProcessor`] (the shared gain DSP law), its context and
/// all per-block scratch.
pub(crate) struct TestStatefulAudioProcessor<'a> {
    shared: &'a TestStatefulShared,
    processor: TestGainProcessor,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `test_stateful` plugin type — a unit struct tying the
/// associated types together, instantiated through the suite's
/// custom [`clack_plugin::entry::Entry`] factory.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TestStatefulClapPlugin;

impl Plugin for TestStatefulClapPlugin {
    type AudioProcessor<'a> = TestStatefulAudioProcessor<'a>;
    type Shared<'a> = TestStatefulShared;
    type MainThread<'a> = TestStatefulMainThread<'a>;

    fn declare_extensions(
        builder: &mut PluginExtensions<Self>,
        _shared: Option<&TestStatefulShared>,
    ) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>()
            .register::<PluginState>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for TestStatefulClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform-test.clap-test-stateful", "test_stateful")
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — persistent gain state for host save/restore verification",
            )
            .with_features([features::AUDIO_EFFECT, features::STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<TestStatefulShared, PluginError> {
        // `plugin.init` — the host handle is intentionally not stored:
        // this fixture declares no host-facing extensions that need
        // it after construction.
        TestStatefulShared::initialize()
    }

    fn new_main_thread<'a>(
        host: HostMainThreadHandle<'a>,
        shared: &'a TestStatefulShared,
    ) -> Result<TestStatefulMainThread<'a>, PluginError> {
        Ok(TestStatefulMainThread {
            shared,
            host: Some(host),
        })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for TestStatefulMainThread<'_> {
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

impl PluginMainThreadParams for TestStatefulMainThread<'_> {
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

impl PluginAudioProcessorParams for TestStatefulAudioProcessor<'_> {
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
// State extension (main thread)
// ---------------------------------------------------------------------------

impl PluginStateImpl for TestStatefulMainThread<'_> {
    /// `clap.ext.state` `save` — writes the fixture's 16-byte blob:
    /// the persisted plain gain, serialized deterministically.
    ///
    /// **RT contract:** non-RT only — runs on the main thread; the
    /// host-provided stream may allocate.
    ///
    /// # Errors
    ///
    /// `PluginError::Message` — the host stream declined the write.
    fn save(&self, output: &mut OutputStream) -> Result<(), PluginError> {
        let blob = encode_state_blob(self.shared.gain_mirror.load());
        output
            .write_all(&blob)
            .map_err(|_| PluginError::Message("state save stream failed"))
    }

    /// `clap.ext.state` `load` — reads exactly one 16-byte blob,
    /// validates it through [`decode_state_blob`], and stores the
    /// decoded plain gain into the mirror. A rejected blob leaves
    /// the mirror untouched.
    ///
    /// **RT contract:** non-RT only — runs on the main thread; the
    /// host-provided stream may allocate.
    ///
    /// # Errors
    ///
    /// `PluginError::Message` — truncated stream, trailing bytes,
    /// stream failure, or a blob failing codec validation.
    fn load(&self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut blob = [0u8; STATE_BLOB_LEN];
        input
            .read_exact(&mut blob)
            .map_err(|_| PluginError::Message("state blob truncated"))?;
        // The fixture's stream carries exactly one blob — trailing
        // bytes are a defined rejection, not ignored input.
        let mut extra = [0u8; 1];
        if input
            .read(&mut extra)
            .map_err(|_| PluginError::Message("state stream failed"))?
            != 0
        {
            return Err(PluginError::Message("state blob has trailing data"));
        }
        let gain = decode_state_blob(&blob).ok_or(PluginError::Message("malformed state blob"))?;
        let previous = self.shared.gain_mirror.load();
        self.shared.gain_mirror.store(gain);
        // CLAP contract: parameter values changed outside param events
        // (a state restore) must be announced through
        // `clap_host_params.rescan` so the host re-reads them — hosts
        // without the params extension simply never see the request.
        if previous.to_bits() != gain.to_bits()
            && let Some(host) = self.host
            && let Some(params) = host.get_extension::<HostParams>()
        {
            params.rescan(&host, ParamRescanFlags::VALUES);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Audio processor
// ---------------------------------------------------------------------------

impl<'a> TestStatefulAudioProcessor<'a> {
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
        shared: &'a TestStatefulShared,
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
        // Unlike `test_gain`'s deterministic per-activation reset,
        // `test_stateful` preserves the mirror across activation —
        // the persisted (possibly `load`-restored) gain is the state
        // under test and must survive re-activation.
        Ok(Self {
            shared,
            processor,
            context,
            config,
            scratch: BlockScratch::new(block::MAX_CHANNELS, block::MAX_EVENTS_PER_BLOCK),
        })
    }
}

impl<'a> PluginAudioProcessor<'a, TestStatefulShared, TestStatefulMainThread<'a>>
    for TestStatefulAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &TestStatefulMainThread<'a>,
        shared: &'a TestStatefulShared,
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
            block::translate_events(events.input, &mut self.scratch, frames_count, |unknown| {
                block::translate_param_event(unknown, &GAIN_PARAM)
            })?;
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result =
            self.processor
                .process(&inputs, &mut outputs, event_list, None, &mut self.context);
        self.shared.gain_mirror.store(self.processor.gain_plain());
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestStatefulMainThread<'a>) {
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
    use std::sync::{Arc, Mutex};

    use clack_common::events::Pckn;
    use clack_common::events::event_types::ParamValueEvent;
    use clack_common::events::io::EventBuffer;
    use clack_extensions::params::{
        HostParams, HostParamsImplMainThread, HostParamsImplShared, ParamClearFlags,
        ParamRescanFlags,
    };
    use clack_host::entry::PluginEntry;
    use clack_host::host::{
        HostExtensions, HostHandlers, HostInfo, MainThreadHandler, SharedHandler,
    };
    use clack_host::plugin::PluginInstance;
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputAudioBuffers, InputChannel,
        OutputAudioBuffers,
    };

    use crate::block::MAX_CHANNELS;
    use ruviform_plugin_core::LifecycleState;

    // `plugin.rs`'s test module owns the crate's single
    // `#[global_allocator]` (`AllocDisabler`) — `assert_no_alloc`
    // below observes allocations through that shared swap.

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
        ap: &mut TestStatefulAudioProcessor<'_>,
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
    fn activated(shared: &TestStatefulShared, max_frames: u32) -> TestStatefulAudioProcessor<'_> {
        TestStatefulAudioProcessor::activate_inner(shared, &config(max_frames)).unwrap()
    }

    /// A stereo `f32` port pair: distinct input/output backing `Vec`s
    /// plus the `AudioPorts` descriptor storage a host would own for
    /// one `process` call — the same fixture shape `plugin.rs`'s
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

    // -- State blob codec ---------------------------------------------------

    #[test]
    fn state_blob_round_trips_plain_gain() {
        let blob = encode_state_blob(0.75);
        assert_eq!(blob.len(), STATE_BLOB_LEN);
        assert_eq!(&blob[0..4], b"RFST");
        assert_eq!(decode_state_blob(&blob), Some(0.75));
        // Default and extremes round-trip exactly.
        for gain in [1.0_f64, 0.0, 2.0] {
            assert_eq!(decode_state_blob(&encode_state_blob(gain)), Some(gain));
        }
    }

    #[test]
    fn state_blob_rejects_malformed_input() {
        let valid = encode_state_blob(1.0);
        // Wrong length — short and long.
        assert_eq!(decode_state_blob(&valid[..8]), None);
        assert_eq!(
            decode_state_blob(&[valid.as_slice(), &[0u8]].concat()),
            None
        );
        // Corrupted magic / version.
        let mut bad_magic = valid;
        bad_magic[0] ^= 0xFF;
        assert_eq!(decode_state_blob(&bad_magic), None);
        let mut bad_version = valid;
        bad_version[7] = 2;
        assert_eq!(decode_state_blob(&bad_version), None);
        // Non-finite payload.
        for bits in [f64::NAN.to_bits(), f64::INFINITY.to_bits()] {
            let mut bad = valid;
            bad[8..16].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(decode_state_blob(&bad), None);
        }
    }

    #[test]
    fn state_blob_clamps_finite_out_of_range() {
        // Finite out-of-range values clamp to the plain range — the
        // `text_to_value` law — rather than rejecting.
        assert_eq!(decode_state_blob(&encode_state_blob(5.0)), Some(2.0));
        assert_eq!(decode_state_blob(&encode_state_blob(-1.0)), Some(0.0));
    }

    // -- State extension verbs (direct, pre-ABI) ----------------------------

    /// `save` writes exactly the codec blob; `load` restores it into
    /// the mirror — the plugin-defined stream contract the host
    /// bridge forwards unchanged.
    #[test]
    fn state_extension_saves_and_loads_plain_gain() {
        let shared = TestStatefulShared::initialize().unwrap();
        shared.gain_mirror.store(0.75);
        let main = TestStatefulMainThread {
            shared: &shared,
            host: None,
        };

        let mut bytes = Vec::new();
        {
            let mut stream = OutputStream::from_writer(&mut bytes);
            main.save(&mut stream).unwrap();
        }
        assert_eq!(bytes, encode_state_blob(0.75));

        shared.gain_mirror.store(1.0);
        let mut reader = std::io::Cursor::new(&bytes);
        {
            let mut stream = InputStream::from_reader(&mut reader);
            main.load(&mut stream).unwrap();
        }
        assert_eq!(shared.gain_mirror.load(), 0.75);
        assert_eq!(main.get_value(ClapId::new(0)), Some(0.75));
    }

    /// `load` rejections are defined errors and leave the mirror
    /// untouched: truncated, trailing bytes, bad magic, non-finite.
    #[test]
    fn state_extension_load_rejects_malformed_blobs() {
        let shared = TestStatefulShared::initialize().unwrap();
        shared.gain_mirror.store(1.0);
        let main = TestStatefulMainThread {
            shared: &shared,
            host: None,
        };

        let try_load = |bytes: &[u8]| {
            let mut reader = std::io::Cursor::new(bytes);
            let mut stream = InputStream::from_reader(&mut reader);
            main.load(&mut stream)
        };

        let valid = encode_state_blob(0.5);
        // Truncated.
        assert!(try_load(&valid[..8]).is_err());
        // Trailing bytes.
        let mut trailing = valid.to_vec();
        trailing.push(0);
        assert!(try_load(&trailing).is_err());
        // Bad magic.
        let mut bad_magic = valid;
        bad_magic[0] ^= 0xFF;
        assert!(try_load(&bad_magic).is_err());
        // Non-finite.
        let mut nan = valid;
        nan[8..16].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
        assert!(try_load(&nan).is_err());

        // Every rejection left the mirror at its prior value.
        assert_eq!(shared.gain_mirror.load(), 1.0);
    }

    // -- Params rescan contract (state load) ---------------------------------

    /// Recording host side: every `clap_host_params.rescan` the plugin
    /// issues lands in `rescans` — the observable half of the CLAP
    /// contract that a state `load` changing parameter values must
    /// announce a values rescan.
    struct RescanProbeShared {
        rescans: Arc<Mutex<Vec<ParamRescanFlags>>>,
    }

    struct RescanProbeMainThread<'a> {
        shared: &'a RescanProbeShared,
    }

    struct RescanProbeHost;

    impl SharedHandler<'_> for RescanProbeShared {
        fn request_restart(&self) {}
        fn request_process(&self) {}
        fn request_callback(&self) {}
    }

    impl HostParamsImplShared for RescanProbeShared {
        fn request_flush(&self) {}
    }

    impl<'a> MainThreadHandler<'a> for RescanProbeMainThread<'a> {}

    impl HostParamsImplMainThread for RescanProbeMainThread<'_> {
        fn rescan(&self, flags: ParamRescanFlags) {
            self.shared
                .rescans
                .lock()
                .expect("rescan recorder poisoned")
                .push(flags);
        }

        fn clear(&self, _param_id: ClapId, _flags: ParamClearFlags) {}
    }

    impl HostHandlers for RescanProbeHost {
        type Shared<'a> = RescanProbeShared;
        type MainThread<'a> = RescanProbeMainThread<'a>;
        type AudioProcessor<'a> = ();

        fn declare_extensions(builder: &mut HostExtensions<Self>, _shared: &Self::Shared<'_>) {
            builder.register::<HostParams>();
        }
    }

    /// `clap.ext.state` `load` runs the real ABI path — instantiate
    /// `test_stateful` through the suite entry via clack-host, restore
    /// a blob that changes the gain, and assert the plugin requested a
    /// `VALUES` params rescan exactly once (and not again when the
    /// restored value is already current).
    #[test]
    fn state_load_requests_params_values_rescan() {
        let rescans = Arc::new(Mutex::new(Vec::new()));
        let entry =
            PluginEntry::load_from_clack::<crate::TestSuiteEntry>(c"stateful.clap").unwrap();
        let host_info = HostInfo::new("test host", "ruviform", "", "0.0.1").unwrap();
        let mut instance = PluginInstance::<RescanProbeHost>::new(
            {
                let rescans = Arc::clone(&rescans);
                move |()| RescanProbeShared { rescans }
            },
            |shared| RescanProbeMainThread { shared },
            &entry,
            c"org.ruviform-test.clap-test-stateful",
            &host_info,
        )
        .unwrap();

        let handle = instance.plugin_handle();
        let state = handle.get_extension::<PluginState>().unwrap();

        // Restore gain 0.5 — different from the 1.0 default → rescan.
        let mut reader = std::io::Cursor::new(encode_state_blob(0.5));
        state.load(&handle, &mut reader).unwrap();
        assert_eq!(
            *rescans.lock().expect("rescan recorder poisoned"),
            vec![ParamRescanFlags::VALUES]
        );

        // Re-load the identical value — nothing changed → no rescan.
        let mut reader = std::io::Cursor::new(encode_state_blob(0.5));
        state.load(&handle, &mut reader).unwrap();
        assert_eq!(rescans.lock().expect("rescan recorder poisoned").len(), 1);
    }

    // -- Activation preserves persisted state -------------------------------

    /// The stateful fixture's defining law: a `load`-written (or
    /// mutated) gain survives `activate` — unlike `test_gain`, whose
    /// activation deterministically resets to the default.
    #[test]
    fn activation_preserves_loaded_gain() {
        let shared = TestStatefulShared::initialize().unwrap();
        let main = TestStatefulMainThread {
            shared: &shared,
            host: None,
        };

        // Restore gain 0.75 through the state surface before activation.
        let mut bytes = Vec::new();
        shared.gain_mirror.store(0.75);
        {
            let mut stream = OutputStream::from_writer(&mut bytes);
            main.save(&mut stream).unwrap();
        }
        shared.gain_mirror.store(1.0);
        let mut reader = std::io::Cursor::new(&bytes);
        {
            let mut stream = InputStream::from_reader(&mut reader);
            main.load(&mut stream).unwrap();
        }
        assert_eq!(shared.gain_mirror.load(), 0.75);

        // Activate — the mirror must NOT reset to the default.
        let mut ap = activated(&shared, FRAMES);
        assert_eq!(shared.gain_mirror.load(), 0.75);
        ap.start_processing().unwrap();

        // And the restored gain drives the DSP on the first block.
        let mut fixture = StereoFixture::new(FRAMES_USIZE);
        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        {
            let (input_buffers, mut output_buffers) = fixture.guards();
            let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
            assert_eq!(
                process_block(&mut ap, audio, &input_buffer, &mut output_buffer).unwrap(),
                ClapProcessStatus::Continue
            );
        }
        assert_eq!(fixture.outputs[0], vec![0.375; FRAMES_USIZE]);
        assert_eq!(fixture.outputs[1], vec![-0.1875; FRAMES_USIZE]);
    }

    // -- Real process path ---------------------------------------------------

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
        let shared = TestStatefulShared::initialize().unwrap();
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
    fn process_applies_param_event_sample_accurately() {
        let shared = TestStatefulShared::initialize().unwrap();
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
        // And the mirror carries the post-block value for params/state.
        assert_eq!(shared.gain_mirror.load(), 0.5);
    }

    // -- Params surface ------------------------------------------------------

    /// The `params` extension's main-thread surface: `get_value`
    /// reports the mirrored plain gain, and `flush` folds `ParamValue`
    /// events into it — observable without a process block.
    #[test]
    fn params_extension_reports_and_flushes_gain() {
        let shared = TestStatefulShared::initialize().unwrap();
        let main = TestStatefulMainThread {
            shared: &shared,
            host: None,
        };
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

    // -- Lifecycle mapping ---------------------------------------------------

    #[test]
    fn lifecycle_happy_path_maps_created_to_destroyed() {
        let shared = TestStatefulShared::initialize().unwrap();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);

        let mut ap = activated(&shared, FRAMES);
        assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);

        ap.start_processing().unwrap();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);

        let input_buffer = EventBuffer::new();
        let mut output_buffer = EventBuffer::new();
        assert_eq!(
            process_block(&mut ap, empty_audio(), &input_buffer, &mut output_buffer).unwrap(),
            ClapProcessStatus::Sleep
        );
        assert_eq!(shared.lifecycle.state(), LifecycleState::Processing);

        ap.stop_processing();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Activated);

        let main = TestStatefulMainThread {
            shared: &shared,
            host: None,
        };
        ap.deactivate(&main); // consumes the audio processor
        assert_eq!(shared.lifecycle.state(), LifecycleState::Initialized);

        shared.destroy();
        assert_eq!(shared.lifecycle.state(), LifecycleState::Destroyed);
    }
}
