//! The VST3 component class: `IComponent` + `IAudioProcessor` +
//! `IProcessContextRequirements` over `ComWrapper`, driving the
//! neutral [`PluginProcessor`] contract through the common
//! [`Lifecycle`] machine.
//!
//! ## Lifecycle mapping (VST3 → neutral events)
//!
//! | VST3 call | `LifecycleEvent` | Notes |
//! | --- | --- | --- |
//! | `initialize` | `Initialize` | `Created → Initialized` |
//! | `setActive(true)` | `Activate` | `Initialized → Activated` |
//! | `setupProcessing` | — | `PluginProcessor::prepare` + scratch alloc (stays `Activated`) |
//! | `setProcessing(true)` | `StartProcessing` | `Activated → Processing`; requires a successful `setupProcessing` |
//! | `process` | `Process` | self-loop; rejected elsewhere → `kResultFalse` |
//! | `setProcessing(false)` | `StopProcessing` | `Processing → Activated` |
//! | `setActive(false)` | `Deactivate` | `Activated → Initialized`; rejected while `Processing` |
//! | `terminate` | `Destroy` | requires prior `Deactivate` |
//!
//! Protocol violations are defined rejections (`kResultFalse`) —
//! e.g. `process` outside `Processing`, `setActive(false)` while
//! `Processing`, `terminate` before `Deactivate`.
//!
//! ## Interior mutability
//!
//! The generated traits take `&self`, so per-instance state lives in a
//! `RefCell<ComponentState>` — a finding recorded for the binding ADR.
//! The VST3 threading contract calls each method on a documented
//! thread with no concurrent access; a protocol-violating re-entrant
//! call on the same thread is a defined `kResultFalse` rejection via
//! `try_borrow_mut` rather than a `RefCell` panic unwinding through
//! `extern "system"` (which would abort the host). Concurrent
//! cross-thread calls remain a host threading-contract violation
//! outside the defended boundary; only `getTailSamples` is documented
//! for any-thread access, and its state is an atomic.
//!
//! ## `unsafe` discipline
//!
//! `unsafe` is localized to the FFI boundary: the trait impls carry
//! `unsafe fn` signatures imposed by the binding and each body uses
//! scoped `unsafe {}` blocks with SAFETY comments. The per-block
//! validation/view building lives in [`crate::block`].

// The generated bindings declare enum constants as `DefaultEnumType`
// (`c_int` on Windows, `c_uint` elsewhere) while struct fields use
// `int32`; the `as` casts below are required on non-Windows platforms
// and a no-op on Windows — suppress `unnecessary_cast` for this file.
#![allow(clippy::unnecessary_cast)]

use std::cell::RefCell;
use std::slice;
use std::sync::atomic::{AtomicU32, Ordering};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{MediaTime, TimeBase, TransportStatus};
use ruviform_plugin_core::{
    Event, EventKind, EventList, Lifecycle, LifecycleEvent, LifecycleState, ParamEvent, ParamId,
    ParamValue, PluginProcessor, ProcessConfig, ProcessContext, ProcessStatus, TransportInfo,
};
use vst3::Steinberg::Vst::{
    BusDirection, BusDirections, BusDirections_, BusInfo, BusInfo_::BusFlags_, BusType, BusTypes_,
    IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentTrait, IParamValueQueueTrait,
    IParameterChanges, IParameterChangesTrait, IProcessContextRequirements,
    IProcessContextRequirementsTrait, IoMode, MediaType, MediaTypes, MediaTypes_, ProcessContext_,
    ProcessData, ProcessSetup, RoutingInfo, SpeakerArr, SpeakerArrangement, SymbolicSampleSizes_,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPluginBaseTrait, TBool, TUID, kInvalidArgument, kNotImplemented,
    kResultFalse, kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComRef, uid};

use crate::block::{self, ChannelScratch, EventScratch, MAX_BLOCK_EVENTS, PreparedBlock};
use crate::controller::GainController;
use crate::gain::{GAIN_PARAM, GainProcessor};
use crate::strings::copy_wstring;

/// The VST3 `ProcessContext` — aliased to keep it distinct from the
/// neutral `ruviform_plugin_core::ProcessContext` used everywhere else.
type VstProcessContext = vst3::Steinberg::Vst::ProcessContext;

/// Channel count negotiated for the single stereo bus pair — the `PoC`
/// plugin only accepts `SpeakerArr::kStereo` (2 channels per bus).
const CHANNELS_PER_BUS: usize = 2;

/// Upper bound on the negotiated `maxSamplesPerBlock` — a defined
/// `setupProcessing` rejection above this `PoC` ceiling keeps a
/// hostile host from driving the per-instance bounce scratch into a
/// multi-GiB allocation (2 channels × 1 MiB frames ≈ 8 MiB).
const MAX_NEGOTIATED_FRAMES: i32 = 1 << 20;

/// Per-instance component state — everything the `&self` COM methods
/// need to mutate, behind one `RefCell`.
struct ComponentState {
    lifecycle: Lifecycle,
    processor: GainProcessor,
    context: ProcessContext,
    /// The negotiated `ProcessConfig` — `Some` after a successful
    /// `setupProcessing`; `setProcessing(true)` requires it.
    config: Option<ProcessConfig>,
    /// Negotiated sample rate, kept for the transport mapping.
    sample_rate: u32,
    channel_scratch: ChannelScratch,
    event_scratch: EventScratch,
}

impl ComponentState {
    fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(),
            processor: GainProcessor::new(),
            context: ProcessContext::new(),
            config: None,
            sample_rate: 0,
            channel_scratch: ChannelScratch::empty(),
            event_scratch: EventScratch::empty(),
        }
    }
}

/// The VST3 component for the `PoC` gain plugin.
///
/// `Interfaces` is the COM query-interface list; VST3's `IComponent`
/// and `IAudioProcessor` both derive from `IPluginBase`, so the
/// `IPluginBase` trait impl supplies `initialize`/`terminate` for all
/// of them.
pub(crate) struct GainComponent {
    state: RefCell<ComponentState>,
    /// Latest tail length reported by the processor —
    /// `getTailSamples` may run on any thread, so it stays an atomic.
    tail: AtomicU32,
}

impl Class for GainComponent {
    type Interfaces = (IComponent, IAudioProcessor, IProcessContextRequirements);
}

impl GainComponent {
    /// The component class ID (`PoC` value — regenerate per real plugin).
    pub(crate) const CID: TUID = uid(0x5A1B_2C3D, 0x4E5F_6A7B, 0x8C9D_0E1F, 0x2A3B_4C5D);

    pub(crate) fn new() -> Self {
        Self {
            state: RefCell::new(ComponentState::new()),
            tail: AtomicU32::new(0),
        }
    }

    /// Reads `IParameterChanges` into `event_scratch`: one
    /// [`ParamEvent`] per gain queue point, out-of-block offsets and
    /// non-finite values dropped, list sorted by offset. The scan is
    /// bounded — at most `MAX_BLOCK_EVENTS` queues and `MAX_BLOCK_EVENTS`
    /// points per queue are visited, and it stops as soon as the
    /// bounded scratch fills (defined truncation).
    ///
    /// # Safety
    ///
    /// `changes` may be null; otherwise it must point to a valid
    /// `IParameterChanges` object for the duration of the call (the
    /// VST3 `process` contract guarantees it for
    /// `data.inputParameterChanges`). Returned queues are borrowed
    /// (`ComRef`) and never stored.
    #[expect(unsafe_code)]
    unsafe fn collect_param_events(
        changes: *mut IParameterChanges,
        frames: usize,
        state: &mut ComponentState,
    ) {
        let events = &mut state.event_scratch;
        events.clear();
        // SAFETY: caller guarantees `changes` is null or a valid
        // `IParameterChanges` for this call — `ComRef` borrows without
        // touching the refcount.
        let Some(changes) = (unsafe { ComRef::from_raw(changes) }) else {
            return;
        };
        // SAFETY: `changes` is a live `IParameterChanges` for the call.
        let count = usize::try_from(unsafe { changes.getParameterCount() }.max(0)).unwrap_or(0);
        // A host may declare arbitrary queue/point counts; the scan
        // stays bounded — at most `MAX_BLOCK_EVENTS` queues and
        // `MAX_BLOCK_EVENTS` points per queue.
        'queues: for index in 0..count.min(MAX_BLOCK_EVENTS) {
            // SAFETY: `index < count` within this live
            // `IParameterChanges`; the returned queue pointer is
            // borrowed for this call only.
            let Some(queue) = (unsafe {
                ComRef::from_raw(changes.getParameterData(i32::try_from(index).unwrap_or(i32::MAX)))
            }) else {
                continue;
            };
            // SAFETY: `queue` is a live `IParamValueQueue` for the call.
            let param_id = ParamId(unsafe { queue.getParameterId() });
            // Foreign parameter ids never enter the scratch — they
            // cannot consume the bounded event budget.
            if param_id != GAIN_PARAM.id {
                continue;
            }
            // SAFETY: `queue` is a live `IParamValueQueue` for the call.
            let points = usize::try_from(unsafe { queue.getPointCount() }.max(0)).unwrap_or(0);
            for point in 0..points.min(MAX_BLOCK_EVENTS) {
                let mut offset = 0_i32;
                let mut value = 0.0_f64;
                // SAFETY: `point < points` within this live queue;
                // `offset`/`value` are valid out-params.
                if unsafe {
                    queue.getPoint(
                        i32::try_from(point).unwrap_or(i32::MAX),
                        &raw mut offset,
                        &raw mut value,
                    )
                } != kResultTrue
                {
                    continue;
                }
                // Defined skips: non-finite values (the `ParamValue`
                // NaN policy) and out-of-block offsets (the shared
                // drop policy — never clamped onto a wrong frame).
                let Ok(value) = ParamValue::new(value) else {
                    continue;
                };
                let Some(offset) = block::block_offset(offset, frames) else {
                    continue;
                };
                let event = Event::new(
                    offset,
                    EventKind::ParamValue(ParamEvent { param_id, value }),
                );
                // Scratch full → every remaining host point is dropped
                // anyway; stop scanning (bounded-work guarantee).
                if !events.push(event) {
                    break 'queues;
                }
            }
        }
        events.sort();
    }

    /// Maps the optional VST3 `ProcessContext` to the neutral
    /// [`TransportInfo`]: `kPlaying` → `Playing`, otherwise `Stopped`;
    /// `projectTimeSamples` → `MediaTime` at the negotiated rate.
    /// Returns `None` when the context is absent or the rate
    /// unset — matching the neutral `Option<TransportInfo>` contract.
    ///
    /// # Safety
    ///
    /// `process_context` must be null or point to a valid
    /// `ProcessContext` for this call (VST3 `process` contract).
    #[expect(unsafe_code)]
    unsafe fn transport(
        process_context: *const VstProcessContext,
        sample_rate: u32,
    ) -> Option<TransportInfo> {
        if process_context.is_null() || sample_rate == 0 {
            return None;
        }
        // SAFETY: caller guarantees the pointer is valid for the call.
        let ctx = unsafe { &*process_context };
        let status = if ctx.state & ProcessContext_::StatesAndFlags_::kPlaying as u32 != 0 {
            TransportStatus::Playing
        } else {
            TransportStatus::Stopped
        };
        TimeBase::sample_rate(sample_rate)
            .ok()
            .map(|base| TransportInfo::new(status, MediaTime::new(ctx.projectTimeSamples, base)))
    }
}

impl IPluginBaseTrait for GainComponent {
    /// `Created → Initialized` on success; a double init is a defined
    /// `kResultFalse` rejection.
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        match state.lifecycle.transition(LifecycleEvent::Initialize) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse,
        }
    }

    /// `Destroy` — legal only from `Initialized`/`Created`, i.e. after
    /// `Deactivate`; a still-activated terminate is rejected.
    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        match state.lifecycle.transition(LifecycleEvent::Destroy) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse,
        }
    }
}

impl IComponentTrait for GainComponent {
    /// Reports the distributed controller class — the host
    /// instantiates `GainController` by this CID.
    #[expect(unsafe_code)]
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        if class_id.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        unsafe { *class_id = GainController::CID };
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    /// One audio input bus, one audio output bus, no event buses.
    #[expect(unsafe_code)]
    unsafe fn getBusCount(&self, media_type: MediaType, dir: BusDirection) -> i32 {
        match media_type as MediaTypes {
            MediaTypes_::kAudio => match dir as BusDirections {
                BusDirections_::kInput | BusDirections_::kOutput => 1,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Describes the single stereo `kMain` bus on each side.
    #[expect(unsafe_code)]
    unsafe fn getBusInfo(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        if bus.is_null() {
            return kInvalidArgument;
        }
        let is_audio = media_type == MediaTypes_::kAudio as MediaType;
        let is_known_dir = dir == BusDirections_::kInput as BusDirection
            || dir == BusDirections_::kOutput as BusDirection;
        if !is_audio || !is_known_dir || index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null; only
        // fields of `BusInfo` are written.
        let bus = unsafe { &mut *bus };
        bus.mediaType = MediaTypes_::kAudio as MediaType;
        bus.direction = dir;
        #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            bus.channelCount = CHANNELS_PER_BUS as i32;
        }
        copy_wstring(
            if dir == BusDirections_::kInput as BusDirection {
                "Input"
            } else {
                "Output"
            },
            &mut bus.name,
        );
        bus.busType = BusTypes_::kMain as BusType;
        bus.flags = BusFlags_::kDefaultActive as u32;
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn getRoutingInfo(
        &self,
        _in_info: *mut RoutingInfo,
        _out_info: *mut RoutingInfo,
    ) -> tresult {
        kNotImplemented
    }

    #[expect(unsafe_code)]
    unsafe fn activateBus(
        &self,
        _media_type: MediaType,
        _dir: BusDirection,
        _index: i32,
        _state: TBool,
    ) -> tresult {
        kResultOk
    }

    /// `setActive(true)` → `Activate`; `setActive(false)` →
    /// `Deactivate` (rejected while `Processing` — the host must stop
    /// processing first).
    #[expect(unsafe_code)]
    unsafe fn setActive(&self, state: TBool) -> tresult {
        let event = if state == 0 {
            LifecycleEvent::Deactivate
        } else {
            LifecycleEvent::Activate
        };
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        match state.lifecycle.transition(event) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse,
        }
    }

    /// State (de)serialization is out of scope for the `PoC` —
    /// `kNotImplemented` honestly reports that instead of a silent
    /// `kResultOk` that would lose the host's state.
    #[expect(unsafe_code)]
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }
}

impl IAudioProcessorTrait for GainComponent {
    /// Accepts exactly the negotiated arrangement: one stereo input +
    /// one stereo output.
    #[expect(unsafe_code)]
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        if num_ins != 1 || num_outs != 1 || inputs.is_null() || outputs.is_null() {
            return kResultFalse;
        }
        // SAFETY: `num_ins`/`num_outs` == 1 and pointers checked
        // non-null — one `SpeakerArrangement` each.
        let (inputs, outputs) = unsafe { (*inputs, *outputs) };
        if inputs == SpeakerArr::kStereo && outputs == SpeakerArr::kStereo {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getBusArrangement(
        &self,
        dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        if arr.is_null() {
            return kInvalidArgument;
        }
        let is_known_dir = dir == BusDirections_::kInput as BusDirection
            || dir == BusDirections_::kOutput as BusDirection;
        if !is_known_dir || index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        unsafe { *arr = SpeakerArr::kStereo };
        kResultOk
    }

    /// f32 only: `kSample64` is declined here *and* re-checked at the
    /// `process` boundary (the spec's "never reinterpret f64 as f32"
    /// rule — 64-bit input is rejected, never reinterpreted).
    #[expect(unsafe_code)]
    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: i32) -> tresult {
        if symbolic_sample_size == SymbolicSampleSizes_::kSample32 as i32 {
            kResultOk
        } else if symbolic_sample_size == SymbolicSampleSizes_::kSample64 as i32 {
            kNotImplemented
        } else {
            kInvalidArgument
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getLatencySamples(&self) -> u32 {
        0
    }

    /// The activation hook (`prepare`) plus scratch allocation: reads
    /// `ProcessSetup`, builds `ProcessConfig`, calls
    /// `PluginProcessor::prepare`, sizes the per-instance scratch.
    /// Legal only while `Activated`; a `kSample64` request is rejected.
    #[expect(unsafe_code)]
    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        if setup.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided `ProcessSetup`, checked non-null.
        let setup = unsafe { &*setup };
        let Ok(mut guard) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        let state = &mut *guard;
        if state.lifecycle.state() != LifecycleState::Activated {
            return kResultFalse;
        }
        if setup.symbolicSampleSize != SymbolicSampleSizes_::kSample32 as i32 {
            return kResultFalse;
        }
        if setup.maxSamplesPerBlock <= 0 || setup.maxSamplesPerBlock > MAX_NEGOTIATED_FRAMES {
            return kResultFalse;
        }
        if !setup.sampleRate.is_finite() || setup.sampleRate < 1.0 {
            return kResultFalse;
        }
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rate = setup.sampleRate.round() as u32;
        let Ok(sample_rate) = SampleRate::new(rate) else {
            return kResultFalse;
        };
        #[expect(clippy::cast_sign_loss)]
        let max_frames = setup.maxSamplesPerBlock as u32;
        let Ok(config) = ProcessConfig::new(sample_rate, 1, max_frames) else {
            return kResultFalse;
        };
        if state
            .processor
            .prepare(&config, &mut state.context)
            .is_err()
        {
            return kResultFalse;
        }
        state.channel_scratch.configure(
            CHANNELS_PER_BUS,
            CHANNELS_PER_BUS,
            usize::try_from(setup.maxSamplesPerBlock).unwrap_or(usize::MAX),
        );
        state.event_scratch.configure();
        state.config = Some(config);
        state.sample_rate = rate;
        kResultOk
    }

    /// `setProcessing(true)` → `StartProcessing` (requires a
    /// successful `setupProcessing`); `setProcessing(false)` →
    /// `StopProcessing` (lands in `Activated`, keeping the prepared
    /// state — deactivation still requires `setActive(false)`).
    #[expect(unsafe_code)]
    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        let event = if state == 0 {
            LifecycleEvent::StopProcessing
        } else {
            LifecycleEvent::StartProcessing
        };
        let Ok(mut component) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        if state != 0 && component.config.is_none() {
            // Defined reject: processing may not start before
            // `setupProcessing` negotiated the config.
            return kResultFalse;
        }
        match component.lifecycle.transition(event) {
            Ok(_) => kResultOk,
            Err(_) => kResultFalse,
        }
    }

    /// The RT path: boundary validation → borrowed views →
    /// `PluginProcessor::process` → bounce writeback → status mapping.
    ///
    /// No allocation, blocking or I/O — the scratch was sized at
    /// `setupProcessing`.
    #[expect(unsafe_code)]
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        if data.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: the VST3 `process` contract guarantees `data` points
        // to a valid `ProcessData` for the duration of the call; we
        // only read it and never retain the pointer.
        let data = unsafe { &*data };

        // Reborrow the `RefMut` once so field accesses split into
        // disjoint borrows (`events` borrows `event_scratch` while
        // `channel_scratch`/`context` are mutably borrowed); the guard
        // stays alive for the whole call.
        let Ok(mut guard) = self.state.try_borrow_mut() else {
            // A re-entrant call (e.g. a host vtable callback into
            // another method while a borrow is held) is a defined
            // rejection, not a `RefCell` panic across the FFI.
            return kResultFalse;
        };
        let state = &mut *guard;
        // Defined reject: `process` is only legal from `Processing`.
        if state.lifecycle.transition(LifecycleEvent::Process).is_err() {
            return kResultFalse;
        }

        // The spec's 64-bit rule: never reinterpret f64 channels as
        // f32 — reject the block instead (`canProcessSampleSize`
        // already declines kSample64; this is the boundary re-check).
        if data.symbolicSampleSize != SymbolicSampleSizes_::kSample32 as i32 {
            return kResultFalse;
        }
        if data.numSamples < 0 {
            return kResultFalse;
        }
        #[expect(clippy::cast_sign_loss)]
        let frames = data.numSamples as usize;
        if frames > state.channel_scratch.max_frames() {
            return kResultFalse;
        }

        // Events + transport are processed even on zero-frame or
        // no-audio blocks (the zero-frame flush semantics of
        // `EventList`).
        // SAFETY: `data.inputParameterChanges` is null or a valid
        // `IParameterChanges` for this call.
        unsafe {
            Self::collect_param_events(data.inputParameterChanges, frames, state);
        }
        // SAFETY: `data.processContext` is null or a valid
        // `ProcessContext` for this call.
        let transport = unsafe { Self::transport(data.processContext, state.sample_rate) };
        let frame_count = u32::try_from(frames).unwrap_or(u32::MAX);
        // `collect_param_events` sorts and drops out-of-block offsets,
        // so `EventList::new` cannot reject; a failure means that
        // invariant broke — a defined reject, never a silently-empty
        // event set.
        let Ok(events) = EventList::new(state.event_scratch.as_slice(), frame_count) else {
            debug_assert!(false, "collected events must satisfy EventList");
            return kResultFalse;
        };

        // A bus pointer must be non-null whenever its count is
        // non-zero — defined reject of a malformed `ProcessData`.
        if (data.numInputs > 0 && data.inputs.is_null())
            || (data.numOutputs > 0 && data.outputs.is_null())
        {
            return kResultFalse;
        }

        let result = if frames == 0 || data.numInputs < 1 || data.numOutputs < 1 {
            // Event-only path: zero frames (the `EventList` flush —
            // offset-0 events apply immediately) or missing buses —
            // no buffer is touched.
            state
                .processor
                .process::<PlanarBufferView, PlanarBufferViewMut>(
                    &[],
                    &mut [],
                    events,
                    transport,
                    &mut state.context,
                )
        } else {
            // SAFETY: `numInputs`/`numOutputs` ≥ 1 and the pointers
            // checked non-null above — the VST3 contract guarantees at
            // least `numInputs`/`numOutputs` entries; only bus 0 is read.
            let in_bus = unsafe { &*data.inputs };
            let out_bus = unsafe { &*data.outputs };
            if usize::try_from(in_bus.numChannels).ok() != Some(CHANNELS_PER_BUS)
                || usize::try_from(out_bus.numChannels).ok() != Some(CHANNELS_PER_BUS)
            {
                return kResultFalse;
            }
            // SAFETY: `channelBuffers32` is the right union member
            // because `symbolicSampleSize == kSample32` was checked
            // above; each bus exposes `numChannels` (== 2) entries per
            // contract. Just reading the pointer, no deref yet.
            let (in_list, out_list) = unsafe {
                (
                    in_bus.__field0.channelBuffers32,
                    out_bus.__field0.channelBuffers32,
                )
            };
            if in_list.is_null() || out_list.is_null() {
                return kResultFalse;
            }
            // SAFETY: non-null pointer lists with `CHANNELS_PER_BUS`
            // entries each (checked above) — the contract guarantees
            // `numChannels` channel pointers per bus for this call.
            let in_ptrs = unsafe { slice::from_raw_parts(in_list, CHANNELS_PER_BUS) };
            let out_ptrs = unsafe { slice::from_raw_parts(out_list, CHANNELS_PER_BUS) };
            match block::prepare_block(in_ptrs, out_ptrs, frames, &mut state.channel_scratch) {
                Err(_) => return kResultFalse,
                Ok(PreparedBlock { input, output }) => {
                    let inputs = [input];
                    let mut outputs = [output];
                    let result = state.processor.process(
                        &inputs,
                        &mut outputs,
                        events,
                        transport,
                        &mut state.context,
                    );
                    // Views are dead after the call (NLL) → the scratch
                    // borrow is released → bounce writeback for
                    // aliased channels.
                    block::finish_block(&mut state.channel_scratch, frames);
                    result
                }
            }
        };

        self.tail.store(result.tail_frames, Ordering::Release);
        match result.status {
            ProcessStatus::Continue | ProcessStatus::Sleep => kResultOk,
            _ => kResultFalse,
        }
    }

    /// Latest tail in frames — the gain processor reports none.
    #[expect(unsafe_code)]
    unsafe fn getTailSamples(&self) -> u32 {
        self.tail.load(Ordering::Acquire)
    }
}

impl IProcessContextRequirementsTrait for GainComponent {
    /// `state`/`projectTimeSamples` are unconditionally valid fields;
    /// no conditionally-valid flags are consumed — the `PoC` requires
    /// nothing.
    #[expect(unsafe_code)]
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vst3::Steinberg::Vst::{
        AudioBusBuffers, AudioBusBuffers__type0, IParamValueQueue, IParamValueQueueTrait,
    };
    use vst3::{ComPtr, ComWrapper};

    /// A minimal `IParamValueQueue` for the param-event path tests.
    struct TestQueue {
        id: u32,
        points: Vec<(i32, f64)>,
    }

    impl Class for TestQueue {
        type Interfaces = (IParamValueQueue,);
    }

    impl IParamValueQueueTrait for TestQueue {
        #[expect(unsafe_code)]
        unsafe fn getParameterId(&self) -> u32 {
            self.id
        }

        #[expect(unsafe_code)]
        unsafe fn getPointCount(&self) -> i32 {
            i32::try_from(self.points.len()).unwrap_or(i32::MAX)
        }

        #[expect(unsafe_code)]
        unsafe fn getPoint(&self, index: i32, sample_offset: *mut i32, value: *mut f64) -> tresult {
            if sample_offset.is_null() || value.is_null() {
                return kResultFalse;
            }
            let Some(&(offset, point)) = self
                .points
                .get(usize::try_from(index).unwrap_or(usize::MAX))
            else {
                return kResultFalse;
            };
            // SAFETY: out-params checked non-null above.
            unsafe {
                *sample_offset = offset;
                *value = point;
            }
            kResultTrue
        }

        #[expect(unsafe_code)]
        unsafe fn addPoint(&self, _sample_offset: i32, _value: f64, _index: *mut i32) -> tresult {
            kNotImplemented
        }
    }

    /// A minimal `IParameterChanges` holding a fixed set of queues.
    struct TestChanges {
        queues: Vec<ComPtr<IParamValueQueue>>,
    }

    impl Class for TestChanges {
        type Interfaces = (IParameterChanges,);
    }

    impl IParameterChangesTrait for TestChanges {
        #[expect(unsafe_code)]
        unsafe fn getParameterCount(&self) -> i32 {
            i32::try_from(self.queues.len()).unwrap_or(i32::MAX)
        }

        #[expect(unsafe_code)]
        unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
            self.queues
                .get(usize::try_from(index).unwrap_or(usize::MAX))
                .map_or(std::ptr::null_mut(), ComPtr::as_ptr)
        }

        #[expect(unsafe_code)]
        unsafe fn addParameterData(
            &self,
            _id: *const u32,
            _index: *mut i32,
        ) -> *mut IParamValueQueue {
            std::ptr::null_mut()
        }
    }

    /// Holds a `ProcessData` bus pair over caller-owned channel
    /// storage — a stand-in for the host-side `AudioBusBuffers` layout.
    struct TestBlock {
        in_channels: Vec<*mut f32>,
        out_channels: Vec<*mut f32>,
        in_bus: AudioBusBuffers,
        out_bus: AudioBusBuffers,
    }

    impl TestBlock {
        fn new(input: &mut [Vec<f32>], output: &mut [Vec<f32>]) -> Self {
            let mut block = Self {
                in_channels: input.iter_mut().map(Vec::as_mut_ptr).collect(),
                out_channels: output.iter_mut().map(Vec::as_mut_ptr).collect(),
                in_bus: AudioBusBuffers {
                    numChannels: 0,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: std::ptr::null_mut(),
                    },
                },
                out_bus: AudioBusBuffers {
                    numChannels: 0,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: std::ptr::null_mut(),
                    },
                },
            };
            #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            {
                block.in_bus.numChannels = block.in_channels.len() as i32;
                block.out_bus.numChannels = block.out_channels.len() as i32;
            }
            // Union field *writes* are safe; only reads need unsafe.
            block.in_bus.__field0.channelBuffers32 = block.in_channels.as_mut_ptr();
            block.out_bus.__field0.channelBuffers32 = block.out_channels.as_mut_ptr();
            block
        }

        fn data(&mut self, frames: i32) -> ProcessData {
            ProcessData {
                processMode: 0,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                numSamples: frames,
                numInputs: 1,
                numOutputs: 1,
                inputs: &raw mut self.in_bus,
                outputs: &raw mut self.out_bus,
                inputParameterChanges: std::ptr::null_mut(),
                outputParameterChanges: std::ptr::null_mut(),
                inputEvents: std::ptr::null_mut(),
                outputEvents: std::ptr::null_mut(),
                processContext: std::ptr::null_mut(),
            }
        }
    }

    /// Drives the full legal handshake: `initialize` → `setActive` →
    /// `setupProcessing` → `setProcessing`.
    ///
    /// # Safety
    ///
    /// `component`/`processor` must be live COM pointers to the same
    /// component for the duration of the call.
    #[expect(unsafe_code)]
    unsafe fn activate(
        component: &ComPtr<IComponent>,
        processor: &ComPtr<IAudioProcessor>,
        max_frames: i32,
    ) {
        // SAFETY: live COM objects per the caller contract.
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                maxSamplesPerBlock: max_frames,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
        }
    }

    /// RT contract exercise: the `process` path over validated host
    /// buffers — `rt_` cannot observe allocation; the no-alloc
    /// guarantee rests on bounded per-block work documented in
    /// `crate::block`.
    #[expect(unsafe_code)]
    #[test]
    fn rt_process_copies_gain_zero_copy() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);

        // SAFETY: `data` + bus/channel pointers valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
        // Unity gain pass-through.
        assert_eq!(output[0], vec![0.5_f32; 64]);
        assert_eq!(output[1], vec![1.0_f32; 64]);
    }

    /// The activate ↔ processing cycle may repeat on one component:
    /// after `setProcessing(0)` + `setActive(0)`, a fresh
    /// `setActive(1)` → `setupProcessing` → `setProcessing(1)`
    /// re-negotiates config and processes again — the shared
    /// `Lifecycle` re-activation transitions.
    #[expect(unsafe_code)]
    #[test]
    fn re_activation_cycles_through_lifecycle() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);

        // SAFETY: `data` + bus/channel pointers valid for the calls.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);

            // Re-activate: fresh `setupProcessing` negotiates a new
            // max-block size and processing resumes.
            assert_eq!(component.setActive(1), kResultOk);
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                maxSamplesPerBlock: 32,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
            let mut small = block.data(32);
            assert_eq!(processor.process(&raw mut small), kResultOk);
            // Blocks larger than the re-negotiated max are rejected.
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
        assert_eq!(output[0][..32], vec![0.5_f32; 32][..]);
    }

    #[expect(unsafe_code)]
    #[test]
    fn process_rejected_outside_processing_state() {
        let component = GainComponent::new();
        let mut data = ProcessData {
            processMode: 0,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
            numSamples: 64,
            numInputs: 0,
            numOutputs: 0,
            inputs: std::ptr::null_mut(),
            outputs: std::ptr::null_mut(),
            inputParameterChanges: std::ptr::null_mut(),
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: std::ptr::null_mut(),
        };
        // SAFETY: `data` is a valid `ProcessData` for the call.
        unsafe {
            assert_eq!(component.process(&raw mut data), kResultFalse);
        }
    }

    #[expect(unsafe_code)]
    #[test]
    fn protocol_violations_are_defined_rejects() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            // Terminating a still-activated component violates the
            // protocol — rejected, and the object stays usable.
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            assert_eq!(component.terminate(), kResultFalse);

            // Double-activate is likewise a defined reject.
            assert_eq!(component.setActive(1), kResultFalse);

            // `setProcessing(true)` before `setupProcessing` is
            // rejected — no config negotiated yet.
            assert_eq!(processor.setProcessing(1), kResultFalse);

            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: SymbolicSampleSizes_::kSample64 as i32,
                maxSamplesPerBlock: 64,
                sampleRate: 48_000.0,
            };
            // kSample64 must be declined at setup and never reaches
            // the buffers.
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultFalse);

            // Still a legal shutdown path after the failures.
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
    }

    #[expect(unsafe_code)]
    #[test]
    fn process_rejects_bad_block_shapes() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);

        // SAFETY: each mutation keeps `data` itself valid; the adapter
        // must reject the bad shape without touching buffers.
        unsafe {
            data.symbolicSampleSize = SymbolicSampleSizes_::kSample64 as i32;
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            data.symbolicSampleSize = SymbolicSampleSizes_::kSample32 as i32;
            data.numSamples = -1;
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            // Above the negotiated `maxSamplesPerBlock`.
            data.numSamples = 65;
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            // Wrong bus channel count.
            data.numSamples = 64;
            data.inputs = std::ptr::null_mut();
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            // Recovered block still processes cleanly.
            data.inputs = &raw mut block.in_bus;
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
    }

    #[expect(unsafe_code)]
    #[test]
    fn in_place_aliased_buffers_bounce_and_write_back() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 32) };

        // In-place: the output bus points at the SAME
        // `AudioBusBuffers` — every output channel pointer equals an
        // input channel pointer.
        let mut input = [vec![0.5_f32; 32], vec![1.0; 32]];
        let mut output = [vec![0.0_f32; 32], vec![0.0; 32]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(32);
        data.outputs = data.inputs;

        // SAFETY: the in-place layout is a legal host shape; the
        // adapter must bounce instead of aliasing `&`/`&mut`.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        // Unity-gain in-place: the input storage holds the result.
        assert_eq!(input[0], vec![0.5_f32; 32]);
        assert_eq!(input[1], vec![1.0_f32; 32]);
    }

    #[expect(unsafe_code)]
    #[test]
    fn param_change_events_reach_the_processor() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let queue_wrapper = ComWrapper::new(TestQueue {
            id: 0,
            points: vec![(0, 0.5)],
        });
        let queue = queue_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();

        let mut input = [vec![1.0_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();

        // SAFETY: `data`, the bus pointers and `changes` are all valid
        // for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        assert_eq!(output[0], vec![0.5_f32; 64]);
        assert_eq!(output[1], vec![0.5_f32; 64]);
    }

    #[expect(unsafe_code)]
    #[test]
    fn zero_frame_block_flushes_offset_zero_events() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let queue_wrapper = ComWrapper::new(TestQueue {
            id: 0,
            points: vec![(0, 0.5)],
        });
        let queue = queue_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();

        // `numSamples == 0` — no buffer touched, but the offset-0 event
        // is due immediately and still applies to the NEXT block.
        let mut data = ProcessData {
            processMode: 0,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
            numSamples: 0,
            numInputs: 0,
            numOutputs: 0,
            inputs: std::ptr::null_mut(),
            outputs: std::ptr::null_mut(),
            inputParameterChanges: changes.as_ptr(),
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: std::ptr::null_mut(),
        };
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }

        let mut input = [vec![1.0_f32; 32], vec![1.0; 32]];
        let mut output = [vec![0.0_f32; 32], vec![0.0; 32]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(32);
        // SAFETY: valid `ProcessData`.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        assert_eq!(output[0], vec![0.5_f32; 32]);
    }

    /// A hostile `IParamValueQueue`: reports `i32::MAX` points and
    /// fails every `getPoint` — the bounded scan must still return.
    struct LyingQueue;

    impl Class for LyingQueue {
        type Interfaces = (IParamValueQueue,);
    }

    impl IParamValueQueueTrait for LyingQueue {
        #[expect(unsafe_code)]
        unsafe fn getParameterId(&self) -> u32 {
            GAIN_PARAM.id.0
        }

        #[expect(unsafe_code)]
        unsafe fn getPointCount(&self) -> i32 {
            i32::MAX
        }

        #[expect(unsafe_code)]
        unsafe fn getPoint(&self, _index: i32, _offset: *mut i32, _value: *mut f64) -> tresult {
            kResultFalse
        }

        #[expect(unsafe_code)]
        unsafe fn addPoint(&self, _offset: i32, _value: f64, _index: *mut i32) -> tresult {
            kNotImplemented
        }
    }

    /// A hostile `IParameterChanges`: reports `i32::MAX` queues, all
    /// null — the queue scan is likewise capped.
    struct LyingChanges;

    impl Class for LyingChanges {
        type Interfaces = (IParameterChanges,);
    }

    impl IParameterChangesTrait for LyingChanges {
        #[expect(unsafe_code)]
        unsafe fn getParameterCount(&self) -> i32 {
            i32::MAX
        }

        #[expect(unsafe_code)]
        unsafe fn getParameterData(&self, _index: i32) -> *mut IParamValueQueue {
            std::ptr::null_mut()
        }

        #[expect(unsafe_code)]
        unsafe fn addParameterData(
            &self,
            _id: *const u32,
            _index: *mut i32,
        ) -> *mut IParamValueQueue {
            std::ptr::null_mut()
        }
    }

    /// Host-declared counts up to `i32::MAX` cannot hang the audio
    /// thread: the scan caps at `MAX_BLOCK_EVENTS` queues/points and
    /// returns.
    #[expect(unsafe_code)]
    #[test]
    fn rt_param_scan_stays_bounded_when_host_counts_lie() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);

        let lying_changes = ComWrapper::new(LyingChanges);
        let changes = lying_changes.to_com_ptr::<IParameterChanges>().unwrap();
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }

        let lying_queue = ComWrapper::new(LyingQueue);
        let queue = lying_queue.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
    }

    /// Queues for parameter ids the plugin does not own are skipped
    /// before their points are scanned — foreign events can never
    /// starve the bounded scratch.
    #[expect(unsafe_code)]
    #[test]
    fn foreign_param_queues_never_reach_the_processor() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let queue_wrapper = ComWrapper::new(TestQueue {
            id: 99,
            points: vec![(0, 0.25)],
        });
        let queue = queue_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        // Unity pass-through: the foreign event was ignored.
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }

    /// An event offset outside the block is dropped, never clamped
    /// onto a wrong frame — the shared drop policy.
    #[expect(unsafe_code)]
    #[test]
    fn out_of_block_event_offsets_are_dropped() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let queue_wrapper = ComWrapper::new(TestQueue {
            id: 0,
            points: vec![(500, 0.25)],
        });
        let queue = queue_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        // The offset-500 event in a 64-frame block is dropped: the
        // gain stays at the default — unity pass-through.
        assert_eq!(output[0], input[0]);
    }

    /// `maxSamplesPerBlock` beyond `MAX_NEGOTIATED_FRAMES` is a
    /// defined reject — the bounce scratch must stay bounded.
    #[expect(unsafe_code)]
    #[test]
    fn setup_processing_rejects_excessive_max_frames() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                maxSamplesPerBlock: i32::MAX,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultFalse);
            // The legal ceiling itself still negotiates.
            setup.maxSamplesPerBlock = MAX_NEGOTIATED_FRAMES;
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
    }

    /// State (de)serialization is out of scope — reported honestly as
    /// `kNotImplemented`, never a silent success.
    #[expect(unsafe_code)]
    #[test]
    fn state_stream_reports_not_implemented() {
        let component = GainComponent::new();
        // SAFETY: null `IBStream` is unused by the stubs.
        unsafe {
            assert_eq!(component.setState(std::ptr::null_mut()), kNotImplemented);
            assert_eq!(component.getState(std::ptr::null_mut()), kNotImplemented);
        }
    }
}
