//! The `test_fault` CLAP plugin: an in-band armed-fault parameter and
//! a pass-through processor that executes the armed fault inside the
//! `process()` call that carries the arming event — the CLAP half of
//! the Issue #630 fault oracle (the VST3 twin lives in
//! `ruviform-plugin-test-vst3`'s `fault.rs`; the law is identical by
//! design so both ABIs present the same contract to the host).
//!
//! ## Fault parameter law
//!
//! `Fault` is parameter id `0` with a plain range of `[0.0, 8.0]`,
//! default `0.0`, stepped (eight modes). The armed modes are reached
//! through ordinary CLAP `ParamValue` events translated by
//! [`block::translate_events`] — no argv, no env vars:
//!
//! | plain value | mode | `process()` behavior |
//! | ----------- | ---- | -------------------- |
//! | `[0.0, 1.0)` | `none` | pass-through only — fully inert |
//! | `[1.0, 2.0)` | `abort` | `std::process::abort()` |
//! | `[2.0, 3.0)` | `exit` | `std::process::exit(code)` — `code = round((plain - 2.0) * 100)` clamped to `[0, 100]` |
//! | `[3.0, 4.0)` | `hang` | never returns (bounded spin) |
//! | `[4.0, 5.0)` | `error` | `CLAP_PROCESS_ERROR` — a defined rejection, not a crash |
//! | `[5.0, 6.0)` | `rt_alloc` | heap-allocates inside `process()` (a bounded 4 KiB `Vec`) and writes the buffer's byte into `output[0][0]` — the `200.0` marker |
//! | `[6.0, 7.0)` | `bad_time` | `CLAP_PROCESS_TAIL` (`TAIL_INFINITE` upstream) plus an output `ParamValue` event stamped `u32::MAX` |
//! | `[7.0, 8.0]` | `invalid` | fills every output sample with NaN, plus an output `ParamValue` event carrying NaN |
//!
//! ## Arming contract
//!
//! Identical to the VST3 fixture: the armed value is the last `Fault`
//! event of the block, persisted across blocks; the mode dispatches
//! at the **end** of the arming block's `process()`. Mode `none`
//! never dispatches — unarmed blocks are pure pass-through.
//!
//! `abort`/`exit`/`hang` intentionally kill or wedge the *host
//! process* — they must only ever run inside the L1 out-of-process
//! child. `error`/`rt_alloc`/`bad_time`/`invalid` stay defined and
//! non-fatal — safe to exercise in-process.
//!
//! ## Detection contract
//!
//! `rt_alloc` is the RT-allocation fault: a bounded heap allocation
//! inside `process()` plus the `output[0][0] = 200.0` marker read
//! *from* the allocated buffer — a host-side allocator cannot observe
//! the fixture's heap (and nothing crosses the L1 process boundary),
//! so the marker is the fixture's own report that the violation ran.
//!
//! `bad_time`/`invalid` are the bad-timestamp / invalid-state faults.
//! The neutral leg is identical to VST3 (`TAIL_INFINITE` →
//! `CLAP_PROCESS_TAIL`; a full-NaN block); the CLAP-only leg emits
//! real ABI output events — a `ParamValue` stamped `u32::MAX` and a
//! `ParamValue` carrying NaN — which a spec-conforming host's event
//! list observes. The ruviform host drains plugin output into
//! canonical reports (Issue #842): the representable `u32::MAX`
//! stamp arrives faithfully, while the NaN payload is
//! unrepresentable in the canonical `ParamValue` and lands as a
//! counted defined skip.

use core::fmt::Write as _;
use std::ffi::CStr;

use clack_common::events::Pckn;
use clack_common::events::event_types::ParamValueEvent;
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
    BufferView, BufferViewMut, EventKind, EventList, LifecycleEvent, ParamFlags, ParamId,
    ParamInfo, PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult,
    ProcessStatus, TransportInfo,
};

use crate::block::{self, AtomicLifecycle, BlockScratch, BlockViews, ParamMirror};

// ---------------------------------------------------------------------------
// Parameter law (shared contract with the VST3 fixture — duplicated per
// fixture so neither crate depends on the other's internals)
// ---------------------------------------------------------------------------

/// CLAP parameter id of the fault parameter.
pub(crate) const FAULT_PARAM_ID: u32 = 0;

/// The `Fault` parameter descriptor — plain range `[0.0, 8.0]`,
/// default `0.0` (`none`), automatable and stepped (eight modes).
pub(crate) const FAULT_PARAM: ParamInfo = ParamInfo {
    id: ParamId(FAULT_PARAM_ID),
    name: "Fault",
    unit: "",
    min_value: 0.0,
    max_value: 8.0,
    default_value: 0.0,
    flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
};

/// The `rt_alloc` mode's heap allocation size — a bounded 4 KiB
/// `Vec` created inside `process()`. The size is fixed so every
/// armed block's churn is identical and bounded.
const RT_ALLOC_CAPACITY: usize = 4096;

/// The byte `rt_alloc` fills its heap buffer with — `f32::from` of it
/// is `200.0`, far outside the legal sample domain. `process()` then
/// writes `f32::from(buf[0])` *read from the buffer* into
/// `output[0][0]`, so the marker proves the heap round-trip ran.
pub(crate) const RT_ALLOC_FILL: u8 = 0xC8;

/// The armed fault mode — the unit band of the plain `Fault` value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FaultMode {
    /// `[0.0, 1.0)` — pass-through only; the unarmed default.
    None,
    /// `[1.0, 2.0)` — `std::process::abort()`.
    Abort,
    /// `[2.0, 3.0)` — `std::process::exit(code)`.
    Exit,
    /// `[3.0, 4.0)` — never returns.
    Hang,
    /// `[4.0, 5.0)` — defined `CLAP_PROCESS_ERROR` rejection.
    Error,
    /// `[5.0, 6.0)` — heap-allocates inside `process()` and marks
    /// `output[0][0]` — the RT-allocation fault.
    RtAlloc,
    /// `[6.0, 7.0)` — claims `TAIL_INFINITE` and emits a
    /// `u32::MAX`-stamped output event — the bad-timestamp fault.
    BadTime,
    /// `[7.0, 8.0]` — fills outputs with NaN and emits a NaN-valued
    /// output event — the invalid-state fault.
    Invalid,
}

/// The `bad_time` mode's CLAP leg — a `ParamValue` output event
/// stamped at `u32::MAX`, beyond any legal block bound: the
/// ABI-level bad timestamp. Shared with `test_editor` (same law).
pub(crate) fn bad_time_output_event() -> ParamValueEvent {
    ParamValueEvent::new(
        u32::MAX,
        ClapId::new(FAULT_PARAM.id.0),
        Pckn::match_all(),
        0.0,
    )
}

/// The `invalid` mode's CLAP leg — a `ParamValue` output event
/// carrying a non-finite value: an ABI-level malformed payload.
/// Shared with `test_editor` (same law).
pub(crate) fn invalid_output_event() -> ParamValueEvent {
    ParamValueEvent::new(
        0,
        ClapId::new(FAULT_PARAM.id.0),
        Pckn::match_all(),
        f64::NAN,
    )
}

/// Decodes a plain `Fault` value into `(mode, exit_code)` — the same
/// law as the VST3 fixture's `decode_fault`.
#[must_use]
pub(crate) fn decode_fault(plain: f64) -> (FaultMode, u32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0.0, 8.0] before the cast — values are small non-negative integers"
    )]
    let index = plain.floor().clamp(0.0, 8.0) as u32;
    let mode = match index {
        0 => FaultMode::None,
        1 => FaultMode::Abort,
        2 => FaultMode::Exit,
        3 => FaultMode::Hang,
        4 => FaultMode::Error,
        5 => FaultMode::RtAlloc,
        6 => FaultMode::BadTime,
        _ => FaultMode::Invalid,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the result is clamped to [0.0, 100.0] before the cast"
    )]
    let exit_code = if mode == FaultMode::Exit {
        (((plain - 2.0) * 100.0).round().clamp(0.0, 100.0)) as u32
    } else {
        0
    };
    (mode, exit_code)
}

/// Copies `input` to `output` channel-for-channel — the trivial
/// pass-through DSP law. Shared with `test_editor` (same law, no
/// armed state).
pub(crate) fn pass_through<I: BufferView, O: BufferViewMut>(
    inputs: &[I],
    outputs: &mut [O],
    frames: usize,
) {
    let Some((input, output)) = inputs.first().zip(outputs.first_mut()) else {
        return;
    };
    let channels = input.channels().min(output.channels());
    for channel in 0..channels {
        let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel)) else {
            continue;
        };
        let limit = frames.min(src.len()).min(dst.len());
        dst[..limit].copy_from_slice(&src[..limit]);
    }
}

/// The `test_fault` processor — pass-through DSP plus the armed-fault
/// dispatch. The armed plain value persists across blocks (the last
/// `Fault` event wins); the mode decodes at the end of every block.
pub(crate) struct TestFaultProcessor {
    /// The armed plain `Fault` value (`[0.0, 8.0]`), held as `f64`.
    armed: f64,
}

impl TestFaultProcessor {
    /// A fresh processor at the parameter default (`armed = 0.0` —
    /// fully inert).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            armed: FAULT_PARAM.default_value,
        }
    }

    /// The current armed plain value — the mirror's sync-out source.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn armed_plain(&self) -> f64 {
        self.armed
    }

    /// Sets the armed plain value — the mirror's sync-in target
    /// (activation reset, `reset`, main-thread `flush`).
    ///
    /// **RT contract:** RT-suitable — pure.
    pub(crate) fn set_armed_plain(&mut self, plain: f64) {
        self.armed = plain;
    }
}

impl PluginProcessor for TestFaultProcessor {
    /// Activation hook — resets the armed value to the parameter
    /// default so every activation starts unarmed.
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.armed = FAULT_PARAM.default_value;
        Ok(())
    }

    /// Runs one block: pass-through copy, armed-value update from the
    /// block's `Fault` events, then the mode dispatch. `abort`/`exit`/
    /// `hang` diverge — they never return a `ProcessResult`. The
    /// `none` path is RT-suitable; the armed paths intentionally
    /// violate RT rules (that is the fault under test).
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let frames = inputs
            .first()
            .map_or(0, BufferView::frames)
            .min(outputs.first().map_or(0, BufferView::frames));
        pass_through(inputs, outputs, frames);

        for event in events.iter() {
            let EventKind::ParamValue(param) = event.kind() else {
                continue;
            };
            if param.param_id != FAULT_PARAM.id {
                continue;
            }
            // Last event wins — the armed value is the final `Fault`
            // point of the block regardless of frame offset.
            self.armed = FAULT_PARAM.denormalize(param.value);
        }

        let result = if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        };
        let (mode, exit_code) = decode_fault(self.armed);
        match mode {
            FaultMode::None => result,
            FaultMode::Error => ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE),
            FaultMode::RtAlloc => {
                // The RT violation under test: a bounded heap
                // allocation inside `process()`. `black_box` keeps the
                // compiler from eliding the round-trip, and the marker
                // is read *from* the buffer so the sample proves the
                // allocation ran.
                let buf = vec![RT_ALLOC_FILL; RT_ALLOC_CAPACITY];
                std::hint::black_box(&buf);
                if let Some(first) = outputs
                    .first_mut()
                    .and_then(|o| o.channel_mut(0))
                    .and_then(|c| c.first_mut())
                {
                    *first = f32::from(buf[0]);
                }
                result
            }
            FaultMode::BadTime => {
                // A zero-tail pass-through fixture claiming an
                // unbounded tail — `map_process_status` reports it as
                // `CLAP_PROCESS_TAIL` downstream.
                ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_INFINITE)
            }
            FaultMode::Invalid => {
                for output in outputs.iter_mut() {
                    for channel in 0..output.channels() {
                        if let Some(data) = output.channel_mut(channel) {
                            data.fill(f32::NAN);
                        }
                    }
                }
                result
            }
            FaultMode::Abort => std::process::abort(),
            FaultMode::Exit => {
                std::process::exit(i32::try_from(exit_code).unwrap_or(i32::MAX));
            }
            FaultMode::Hang => loop {
                std::hint::spin_loop();
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the fault parameter mirror.
pub(crate) struct TestFaultShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The fault parameter's current plain (armed) value, mirrored
    /// for the main-thread `params` extension.
    fault_mirror: ParamMirror,
}

impl PluginShared<'_> for TestFaultShared {}

impl TestFaultShared {
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
            fault_mirror: ParamMirror::new(FAULT_PARAM.default_value),
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

impl Drop for TestFaultShared {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port and
/// parameter metadata live here.
pub(crate) struct TestFaultMainThread<'a> {
    shared: &'a TestFaultShared,
}

impl<'a> PluginMainThread<'a, TestFaultShared> for TestFaultMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`TestFaultProcessor`], its context and all per-block scratch.
pub(crate) struct TestFaultAudioProcessor<'a> {
    shared: &'a TestFaultShared,
    processor: TestFaultProcessor,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `test_fault` plugin type — a unit struct tying the associated
/// types together, instantiated through the suite's custom
/// [`clack_plugin::entry::Entry`] factory.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TestFaultClapPlugin;

impl Plugin for TestFaultClapPlugin {
    type AudioProcessor<'a> = TestFaultAudioProcessor<'a>;
    type Shared<'a> = TestFaultShared;
    type MainThread<'a> = TestFaultMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&TestFaultShared>) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for TestFaultClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform-test.clap-test-fault", "test_fault")
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — in-band armed fault for host failure verification",
            )
            .with_features([features::AUDIO_EFFECT, features::STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<TestFaultShared, PluginError> {
        TestFaultShared::initialize()
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a TestFaultShared,
    ) -> Result<TestFaultMainThread<'a>, PluginError> {
        Ok(TestFaultMainThread { shared })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for TestFaultMainThread<'_> {
    fn count(&self, _is_input: bool) -> u32 {
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
// Params extension
// ---------------------------------------------------------------------------

impl PluginMainThreadParams for TestFaultMainThread<'_> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }
        info.set(&clack_extensions::params::ParamInfo {
            id: ClapId::new(FAULT_PARAM.id.0),
            flags: ParamInfoFlags::IS_AUTOMATABLE
                | ParamInfoFlags::IS_STEPPED
                | ParamInfoFlags::REQUIRES_PROCESS,
            cookie: Cookie::empty(),
            name: FAULT_PARAM.name.as_bytes(),
            module: b"",
            min_value: FAULT_PARAM.min_value,
            max_value: FAULT_PARAM.max_value,
            default_value: FAULT_PARAM.default_value,
        });
    }

    fn get_value(&self, param_id: ClapId) -> Option<f64> {
        (param_id.get() == FAULT_PARAM.id.0).then(|| self.shared.fault_mirror.load())
    }

    fn value_to_text(
        &self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        if param_id.get() != FAULT_PARAM.id.0 {
            return Err(core::fmt::Error);
        }
        write!(writer, "{value:.2}")
    }

    fn text_to_value(&self, param_id: ClapId, text: &CStr) -> Option<f64> {
        if param_id.get() != FAULT_PARAM.id.0 {
            return None;
        }
        let text = text.to_str().ok()?.trim();
        let value: f64 = text.parse().ok()?;
        value
            .is_finite()
            .then(|| value.clamp(FAULT_PARAM.min_value, FAULT_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Main-thread flush: fold every ParamValue for our param into
        // the mirror; offsets carry no meaning outside a process
        // block. The scan is bounded like the process path's.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &FAULT_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .fault_mirror
                    .store(FAULT_PARAM.denormalize(param_event.value));
            }
        }
    }
}

impl PluginAudioProcessorParams for TestFaultAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Audio-thread flush: mirror the change and resync the live
        // processor — CLAP does not guarantee offsets here.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &FAULT_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .fault_mirror
                    .store(FAULT_PARAM.denormalize(param_event.value));
            }
        }
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
    }
}

// ---------------------------------------------------------------------------
// Audio processor
// ---------------------------------------------------------------------------

impl<'a> TestFaultAudioProcessor<'a> {
    /// The body of [`PluginAudioProcessor::activate`], minus the host
    /// handles the trait signature carries — kept as one authoritative
    /// body so tests drive the same `Activate`/`ActivateFailed`
    /// mapping the ABI callback does.
    ///
    /// **RT contract:** non-RT only — allocates per-block scratch.
    ///
    /// # Errors
    ///
    /// As [`PluginAudioProcessor::activate`]: config or preparation
    /// failure reports `ActivateFailed`; a lifecycle rejection is a
    /// host protocol violation.
    fn activate_inner(
        shared: &'a TestFaultShared,
        audio_config: &PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = block::to_process_config(audio_config)
            .map_err(|_| fail("unsupported audio configuration"))?;
        let mut processor = TestFaultProcessor::new();
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
        // Activation resets the armed value to the default (the
        // fixture's deterministic per-activation law).
        shared.fault_mirror.store(FAULT_PARAM.default_value);
        Ok(Self {
            shared,
            processor,
            context,
            config,
            scratch: BlockScratch::new(block::MAX_CHANNELS, block::MAX_EVENTS_PER_BLOCK),
        })
    }
}

impl<'a> PluginAudioProcessor<'a, TestFaultShared, TestFaultMainThread<'a>>
    for TestFaultAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &TestFaultMainThread<'a>,
        shared: &'a TestFaultShared,
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
        // Lifecycle gate — `Process` is illegal outside `Processing`.
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
        // sync-out — keeps `params.value` honest on the main thread.
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
        let event_list =
            block::translate_events(events.input, &mut self.scratch, frames_count, |unknown| {
                block::translate_param_event(unknown, &FAULT_PARAM)
            })?;
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result =
            self.processor
                .process(&inputs, &mut outputs, event_list, None, &mut self.context);
        self.shared.fault_mirror.store(self.processor.armed_plain());
        // The CLAP-only fault legs: `bad_time` emits a `ParamValue`
        // stamped beyond any legal block bound, `invalid` one
        // carrying NaN — real ABI-level malformed events a spec host
        // observes on its output list. The ruviform host drains them
        // into canonical reports (Issue #842); the push stays
        // best-effort and its failure is ignored.
        match decode_fault(self.processor.armed_plain()).0 {
            FaultMode::BadTime => {
                let _ = events.output.try_push(bad_time_output_event());
            }
            FaultMode::Invalid => {
                let _ = events.output.try_push(invalid_output_event());
            }
            _ => {}
        }
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestFaultMainThread<'a>) {
        let _ = self.shared.lifecycle.transition(LifecycleEvent::Deactivate);
    }

    fn reset(&mut self) {
        // CLAP `reset` clears processing state but not parameter
        // values — the armed value *is* the parameter value, so reset
        // resyncs from the authoritative mirror.
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
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

#[cfg(test)]
mod tests {
    use super::*;
    use clack_common::events::io::EventBuffer;
    use clack_common::events::spaces::CoreEventSpace;
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputAudioBuffers, InputChannel,
        OutputAudioBuffers,
    };
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, ParamEvent, ParamValue};

    fn fault_event(normalized: f64) -> Event {
        Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: FAULT_PARAM.id,
                value: ParamValue::new(normalized).unwrap(),
            }),
        )
    }

    /// Runs one block through the neutral contract — only `none`/
    /// `error` modes are exercised here: `abort`/`exit`/`hang` would
    /// kill or wedge the test binary itself.
    fn run_block(
        processor: &mut TestFaultProcessor,
        input: &[Vec<f32>],
        output: &mut [Vec<f32>],
        events: &[Event],
    ) -> ProcessResult {
        let frames = u32::try_from(input.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(events, frames).unwrap();
        let mut context = ProcessContext::new();
        processor.process(&inputs, &mut outputs, list, None, &mut context)
    }

    /// `decode_fault` maps every plain band to its mode; `exit` codes
    /// decode from the fractional band; out-of-domain values clamp.
    #[test]
    fn decode_fault_maps_bands_and_exit_codes() {
        assert_eq!(decode_fault(0.0), (FaultMode::None, 0));
        assert_eq!(decode_fault(0.99), (FaultMode::None, 0));
        assert_eq!(decode_fault(1.0), (FaultMode::Abort, 0));
        assert_eq!(decode_fault(1.99), (FaultMode::Abort, 0));
        assert_eq!(decode_fault(2.0), (FaultMode::Exit, 0));
        assert_eq!(decode_fault(2.42), (FaultMode::Exit, 42));
        assert_eq!(decode_fault(2.99), (FaultMode::Exit, 99));
        assert_eq!(decode_fault(3.0), (FaultMode::Hang, 0));
        assert_eq!(decode_fault(3.99), (FaultMode::Hang, 0));
        assert_eq!(decode_fault(4.0), (FaultMode::Error, 0));
        assert_eq!(decode_fault(4.99), (FaultMode::Error, 0));
        assert_eq!(decode_fault(5.0), (FaultMode::RtAlloc, 0));
        assert_eq!(decode_fault(5.99), (FaultMode::RtAlloc, 0));
        assert_eq!(decode_fault(6.0), (FaultMode::BadTime, 0));
        assert_eq!(decode_fault(6.99), (FaultMode::BadTime, 0));
        assert_eq!(decode_fault(7.0), (FaultMode::Invalid, 0));
        assert_eq!(decode_fault(7.99), (FaultMode::Invalid, 0));
        assert_eq!(decode_fault(8.0), (FaultMode::Invalid, 0));
        assert_eq!(decode_fault(-1.0).0, FaultMode::None);
        assert_eq!(decode_fault(9.0).0, FaultMode::Invalid);
    }

    /// The unarmed default is pure pass-through — `out == in`
    /// bitwise, status `Continue`.
    #[test]
    fn unarmed_block_is_pass_through() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let result = run_block(&mut processor, &input, &mut output, &[]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }

    /// The `error` mode returns `ProcessStatus::Error` — a defined
    /// rejection the host observes as a failed result, not a crash.
    #[test]
    fn error_mode_returns_process_error() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 4.0 → normalized 0.5 → `error`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.5)]);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    /// `rt_alloc` allocates inside `process()` and writes the heap
    /// byte (`200.0` = `f32::from(0xC8)` — `RT_ALLOC_FILL`) into
    /// `output[0][0]` — the sample proves the allocation round-trip
    /// ran. The rest of the block stays pass-through.
    #[test]
    fn alloc_fault_marks_armed_block() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 5.0 → normalized 0.625 → `rt_alloc`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.625)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0][0], f32::from(RT_ALLOC_FILL));
        assert_eq!(output[0][1..], input[0][1..]);
        assert_eq!(output[1], input[1]);
    }

    /// `bad_time` returns `Continue` with `TAIL_INFINITE` — the
    /// zero-tail fixture's timing lie surfaces on the neutral
    /// `tail_frames` a host reads back.
    #[test]
    fn bad_time_mode_claims_infinite_tail() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 6.0 → normalized 0.75 → `bad_time`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.75)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(result.tail_frames, ProcessResult::TAIL_INFINITE);
    }

    /// `invalid` fills every output sample with NaN — the invalid
    /// audio state the host oracle detects on the delivered samples.
    #[test]
    fn invalid_mode_fills_nan_outputs() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert!(output[0].iter().all(|s| s.is_nan()));
        assert!(output[1].iter().all(|s| s.is_nan()));
    }

    /// The armed modes persist until disarmed: after `invalid`, a
    /// `none` event restores pure pass-through — the fault mode's
    /// recovery half.
    #[test]
    fn disarm_restores_pass_through() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        // plain 0.0 → normalized 0.0 → `none`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.0)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    /// RT proof: the unarmed/`none` path never allocates — the armed
    /// `rt_alloc` path is the deliberate violation; the inert law is
    /// untouched. Views and the event list are built outside the
    /// `assert_no_alloc` window so only `process()` itself is
    /// measured (the `rt_` name selects this for `mise run test-rt`).
    #[test]
    fn rt_unarmed_block_is_alloc_free() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let events = [fault_event(0.0)];
        let list = EventList::new(&events, 8).unwrap();
        let mut context = ProcessContext::new();
        assert_no_alloc::assert_no_alloc(|| {
            processor.process(&inputs, &mut outputs, list, None, &mut context)
        });
    }

    /// Foreign parameter ids never arm the fault.
    #[test]
    fn foreign_param_events_do_not_arm() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let foreign = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(9),
                value: ParamValue::new(1.0).unwrap(),
            }),
        );
        let result = run_block(&mut processor, &input, &mut output, &[foreign]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    // -- ABI-level output events (the CLAP-only legs) --------------------

    /// The stereo port fixture a host builds for one `clap_process`
    /// call — `clack_host`'s safe buffer builders over owned `Vec`s,
    /// the same shape as `plugin.rs`'s `StereoFixture`.
    struct StereoFixture {
        inputs: [Vec<f32>; block::MAX_CHANNELS],
        outputs: [Vec<f32>; block::MAX_CHANNELS],
        input_ports: AudioPorts,
        output_ports: AudioPorts,
    }

    impl StereoFixture {
        fn new(frames: usize) -> Self {
            Self {
                inputs: [vec![0.5; frames], vec![-0.25; frames]],
                outputs: [vec![0.0; frames], vec![0.0; frames]],
                input_ports: AudioPorts::with_capacity(block::MAX_CHANNELS, 1),
                output_ports: AudioPorts::with_capacity(block::MAX_CHANNELS, 1),
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

    /// An initialized shared plus an activated audio processor —
    /// the legal path every ABI-level test starts from.
    fn activated(shared: &TestFaultShared) -> TestFaultAudioProcessor<'_> {
        let config = PluginAudioConfiguration {
            sample_rate: 48_000.0,
            min_frames_count: 1,
            max_frames_count: 64,
        };
        TestFaultAudioProcessor::activate_inner(shared, &config).unwrap()
    }

    /// One `process` call through the ABI-level
    /// [`TestFaultAudioProcessor`] — the shape a host builds for
    /// `clap_process` (`Process`/`Events` plumbing plus the event
    /// buffers).
    fn process_abi(
        ap: &mut TestFaultAudioProcessor<'_>,
        fixture: &mut StereoFixture,
        input_buffer: &EventBuffer,
        output_buffer: &mut EventBuffer,
    ) -> ClapProcessStatus {
        let (input_buffers, mut output_buffers) = fixture.guards();
        let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
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
        .unwrap()
    }

    /// The `ParamValue` events the plugin emitted into the output
    /// buffer — `(time, value)` pairs.
    fn emitted_param_values(output_buffer: &EventBuffer) -> Vec<(u32, f64)> {
        output_buffer
            .as_input()
            .iter()
            .filter_map(|unknown| match unknown.as_core_event() {
                Some(CoreEventSpace::ParamValue(event)) => {
                    Some((unknown.header().time(), event.value()))
                }
                _ => None,
            })
            .collect()
    }

    /// `bad_time` through the real ABI: a plain-`6.0` `ParamValue`
    /// input event arms the mode, `process` reports
    /// `CLAP_PROCESS_TAIL`, and the output event list carries a
    /// `ParamValue` stamped `u32::MAX` — the ABI-level bad timestamp
    /// a spec host observes.
    #[test]
    fn bad_time_mode_emits_tail_and_bad_time_event() {
        let shared = TestFaultShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            6.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Tail);
        let emitted = emitted_param_values(&output_buffer);
        assert!(
            emitted.iter().any(|&(time, _)| time == u32::MAX),
            "bad_time must emit a u32::MAX-stamped event, got {emitted:?}"
        );
    }

    /// `invalid` through the real ABI: the output audio is all NaN
    /// and the output event list carries a `ParamValue` event whose
    /// value is non-finite — the ABI-level malformed payload.
    #[test]
    fn invalid_mode_emits_nan_and_invalid_event() {
        let shared = TestFaultShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            7.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Continue);
        assert!(fixture.outputs[0].iter().all(|s| s.is_nan()));
        assert!(fixture.outputs[1].iter().all(|s| s.is_nan()));
        let emitted = emitted_param_values(&output_buffer);
        assert!(
            emitted.iter().any(|&(_, value)| value.is_nan()),
            "invalid must emit a NaN-valued event, got {emitted:?}"
        );
    }

    /// `rt_alloc` through the real ABI: the heap marker lands in the
    /// delivered output buffer — `outputs[0][0] == 200.0` while the
    /// rest of the block stays pass-through.
    #[test]
    fn alloc_fault_marks_armed_abi_output() {
        let shared = TestFaultShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            5.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Continue);
        assert_eq!(fixture.outputs[0][0], f32::from(RT_ALLOC_FILL));
        assert_eq!(fixture.outputs[0][1..], fixture.inputs[0][1..]);
        assert_eq!(fixture.outputs[1], fixture.inputs[1]);
    }
}
