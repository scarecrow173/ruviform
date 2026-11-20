//! The VST3 component class: `IComponent` + `IAudioProcessor` +
//! `IProcessContextRequirements` over `ComWrapper`, driving the
//! neutral [`PluginProcessor`] contract through the common
//! [`Lifecycle`] machine.
//!
//! ## Lifecycle mapping (VST3 → neutral events)
//!
//! | VST3 call | `LifecycleEvent` | Notes |
//! | --- | --- | --- |
//! | `initialize` | `Initialize` | `Created → Initialized`; legal again after `terminate` |
//! | `setActive(true)` | `Activate` | `Initialized → Activated` |
//! | `setupProcessing` | — | `PluginProcessor::prepare` + scratch alloc; legal from `Initialized`/`Activated` |
//! | `setProcessing(true)` | `StartProcessing` | `Activated → Processing`; requires a successful `setupProcessing` |
//! | `process` | `Process` | self-loop; rejected elsewhere → `kResultFalse` |
//! | `setProcessing(false)` | `StopProcessing` | `Processing → Activated` |
//! | `setActive(false)` | `Deactivate` | `Activated → Initialized`; rejected while `Processing` |
//! | `terminate` | `Destroy` then reset | requires prior `Deactivate`; non-terminal — returns to `Created` (VST3 `terminate` ≠ CLAP `destroy`) |
//!
//! Protocol violations are defined rejections (`kResultFalse`) —
//! e.g. `process` outside `Processing`, `setActive(false)` while
//! `Processing`, `terminate` before `Deactivate`.
//!
//! ## Bus / parameter surface
//!
//! One stereo `kMain` audio input bus and one stereo `kMain` audio
//! output bus are declared (`getBusCount`/`getBusInfo`).
//! `setBusArrangements` accepts matching mono or stereo arrangements
//! and `process` accepts 1–2 channels per bus — the gain law is
//! per-channel, so channel count is not part of the contract.
//! `test_instrument` is the suite's only `MidiTransport` input
//! surface: it additionally drains `IEventList` MIDI into the shared
//! event scratch and maps `ProcessContext` to a transport snapshot
//! (see [`InputSurface`]); every other class keeps `ParamsOnly` —
//! `inputEvents`/`processContext` ignored, requirements `0`.
//! State (de)serialization reports
//! `kNotImplemented` honestly on classes without a
//! [`ComponentStateSurface`] — `test_stateful` is the suite's only
//! state surface.
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

// The generated bindings declare flag enum constants as
// `DefaultEnumType` (`c_int` on Windows, `c_uint` elsewhere) while the
// bitfields they write are `uint32`; the remaining `as` casts below are
// required on Windows and a no-op elsewhere — suppress
// `unnecessary_cast` for this file. Enum constants feeding `int32`
// fields or params go through `crate::sdk_enum_i32` instead.
#![allow(clippy::unnecessary_cast)]

use std::cell::RefCell;
use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_plugin_core::{
    Event, EventKind, EventList, Lifecycle, LifecycleEvent, LifecycleState, ParamEvent, ParamId,
    ParamValue, PluginProcessor, ProcessConfig, ProcessContext, ProcessStatus, TransportInfo,
};
use vst3::Steinberg::Vst::{
    BusDirection, BusDirections_, BusInfo, BusInfo_::BusFlags_, BusTypes_, IAudioProcessor,
    IAudioProcessorTrait, IComponent, IComponentTrait, IEventList, IParamValueQueueTrait,
    IParameterChanges, IParameterChangesTrait, IProcessContextRequirements,
    IProcessContextRequirementsTrait, IoMode, MediaType, MediaTypes_,
    ProcessContext as VstProcessContext, ProcessContext_, ProcessData, ProcessSetup, RoutingInfo,
    SpeakerArr, SpeakerArrangement, SymbolicSampleSizes_,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPluginBaseTrait, TBool, TUID, kInvalidArgument, kNotImplemented,
    kResultFalse, kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComRef, uid};

use crate::block::{self, ChannelScratch, EventScratch, MAX_BLOCK_EVENTS, PreparedBlock};
use crate::gain::{GAIN_PARAM, TestGainProcessor};
use crate::sdk_enum_i32;
use crate::stateful::{stream_read_all, stream_write_all};
use crate::strings::copy_wstring;

/// Channel count declared for the single `kMain` bus pair — stereo.
/// `process` additionally accepts 1 channel per bus (mono); the
/// scratch is sized to this capacity.
const CHANNELS_PER_BUS: usize = 2;

/// Upper bound on the negotiated `maxSamplesPerBlock` — a defined
/// `setupProcessing` rejection above this ceiling keeps a hostile
/// host from driving the per-instance bounce scratch into a
/// multi-GiB allocation (2 channels × 1 MiB frames ≈ 8 MiB).
const MAX_NEGOTIATED_FRAMES: i32 = 1 << 20;

/// Total `IParameterChanges` scan budget per block — queue visits and
/// `getPoint` inspections combined. Bounds the *scan* work itself, not
/// just the accepted event count: without it a host declaring
/// `MAX_BLOCK_EVENTS` queues × `MAX_BLOCK_EVENTS` points each could
/// force ~1M inspections while almost no events are accepted.
/// 8 × `MAX_BLOCK_EVENTS` leaves generous headroom for legitimate
/// automation while keeping the worst case in the low-thousands of
/// cheap FFI calls.
const MAX_PARAM_SCAN_OPERATIONS: usize = 8 * MAX_BLOCK_EVENTS;

/// Per-instance component state — everything the `&self` COM methods
/// need to mutate, behind one `RefCell`.
struct ComponentState<P: PluginProcessor> {
    lifecycle: Lifecycle,
    processor: P,
    context: ProcessContext,
    /// The plugin's declared parameter id — `collect_param_events`
    /// admits only this id's queue points into the bounded scratch.
    param_id: ParamId,
    /// The negotiated `ProcessConfig` — `Some` after a successful
    /// `setupProcessing`; `setProcessing(true)` requires it.
    config: Option<ProcessConfig>,
    channel_scratch: ChannelScratch,
    event_scratch: EventScratch,
    /// Negotiated channel count per bus (`setBusArrangements`);
    /// defaults to the declared stereo. `process` accepts 1–2
    /// regardless — the gain law is channel-count agnostic.
    bus_channels: usize,
}

impl<P: PluginProcessor> ComponentState<P> {
    fn new(processor: P, param_id: ParamId) -> Self {
        Self {
            lifecycle: Lifecycle::new(),
            processor,
            context: ProcessContext::new(),
            param_id,
            config: None,
            channel_scratch: ChannelScratch::empty(),
            event_scratch: EventScratch::empty(),
            bus_channels: CHANNELS_PER_BUS,
        }
    }
}

/// A component state surface — the real `IComponent::setState`/
/// `getState` payload contract. Implemented by the stateful
/// fixture's shared gain cell ([`crate::stateful::SharedGain`]);
/// classes without a state surface keep the honest
/// `kNotImplemented` reject instead of a silent `kResultOk` that
/// would lose the host's state.
pub(crate) trait ComponentStateSurface: Send + Sync {
    /// Decodes the component-payload blob into live state —
    /// `false` is the defined rejection (→ `kResultFalse`).
    fn set_component_state(&self, bytes: &[u8]) -> bool;
    /// Serializes live state — deterministic component-payload
    /// bytes for `getState`.
    fn component_state_blob(&self) -> Vec<u8>;
}

/// What a class consumes beyond parameter events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputSurface {
    /// Parameter events only; `inputEvents`/`processContext` are
    /// ignored and no process-context requirements are declared —
    /// every class except `test_instrument`.
    ParamsOnly,
    /// Parameter events plus `IEventList` MIDI input and the
    /// `ProcessContext` transport snapshot — `test_instrument` only.
    MidiTransport,
}

/// The VST3 component shell shared by every test-suite plugin class —
/// `IComponent` + `IAudioProcessor` + `IProcessContextRequirements`
/// over `ComWrapper`, driving the neutral [`PluginProcessor`] contract
/// through the common [`Lifecycle`] machine. The DSP law and parameter
/// surface are the type parameter's own (`TestGainProcessor` /
/// `TestFaultProcessor`); the shell — lifecycle mapping, bus contract,
/// boundary validation, bounded event collection and scratch — is
/// identical for every class, so it lives here once.
///
/// `Interfaces` is the COM query-interface list; VST3's `IComponent`
/// and `IAudioProcessor` both derive from `IPluginBase`, so the
/// `IPluginBase` trait impl supplies `initialize`/`terminate` for all
/// of them.
pub(crate) struct TestComponent<P: PluginProcessor> {
    state: RefCell<ComponentState<P>>,
    /// Latest tail length reported by the processor —
    /// `getTailSamples` may run on any thread, so it stays an atomic.
    tail: AtomicU32,
    /// The distributed controller class ID this component reports
    /// from `getControllerClassId` — each plugin class pairs with its
    /// own controller CID.
    controller_cid: TUID,
    /// The class's state surface — `Some` only for `test_stateful`;
    /// `None` keeps `setState`/`getState` at the honest
    /// `kNotImplemented`.
    state_surface: Option<Arc<dyn ComponentStateSurface>>,
    /// The class's input surface — `MidiTransport` only for
    /// `test_instrument`.
    input_surface: InputSurface,
}

/// `test_gain`'s component — the shared [`TestComponent`] shell over
/// [`TestGainProcessor`]. The associated `CID`/`new` keep the
/// pre-genericization surface (`TestGainComponent::CID`,
/// `TestGainComponent::new()`) resolving through the alias.
pub(crate) type TestGainComponent = TestComponent<TestGainProcessor>;

impl<P: PluginProcessor> Class for TestComponent<P> {
    type Interfaces = (IComponent, IAudioProcessor, IProcessContextRequirements);
}

impl TestComponent<TestGainProcessor> {
    /// The component class ID — a fresh `uid` for the test suite,
    /// distinct from the adapter fixture's `GainComponent::CID` so the
    /// two modules coexist in one host scan.
    pub(crate) const CID: TUID = uid(0x6D4F_8A21, 0x3B7C_9E05, 0xF1A2_4C68, 0xB9D3_7E51);

    pub(crate) fn new() -> Self {
        Self::with(
            crate::controller::GAIN_CONTROLLER_CID,
            GAIN_PARAM.id,
            TestGainProcessor::new(),
        )
    }
}

impl<P: PluginProcessor> TestComponent<P> {
    /// A component over `processor`, reporting `controller_cid` from
    /// `getControllerClassId` and admitting `param_id`'s queue points.
    /// No state surface — `setState`/`getState` stay `kNotImplemented`.
    pub(crate) fn with(controller_cid: TUID, param_id: ParamId, processor: P) -> Self {
        Self {
            state: RefCell::new(ComponentState::new(processor, param_id)),
            tail: AtomicU32::new(0),
            controller_cid,
            state_surface: None,
            input_surface: InputSurface::ParamsOnly,
        }
    }

    /// A component with a live state surface — `test_stateful`'s
    /// constructor path. `surface` owns the persistent state the
    /// processor and the state verbs share.
    pub(crate) fn with_state<S>(
        controller_cid: TUID,
        param_id: ParamId,
        processor: P,
        surface: S,
    ) -> Self
    where
        S: ComponentStateSurface + 'static,
    {
        Self {
            state: RefCell::new(ComponentState::new(processor, param_id)),
            tail: AtomicU32::new(0),
            controller_cid,
            state_surface: Some(Arc::new(surface)),
            input_surface: InputSurface::ParamsOnly,
        }
    }

    /// Marks the component's input surface — `test_instrument` is the
    /// only `MidiTransport` class; every other class keeps the
    /// `ParamsOnly` default.
    pub(crate) fn with_surface(mut self, surface: InputSurface) -> Self {
        self.input_surface = surface;
        self
    }

    /// The shared lifecycle state — a read-only probe for wrappers
    /// that layer class-specific contract strictness on the shared
    /// machine (`test_stateful_single`'s strict `terminate`). A
    /// borrow conflict reports `Failed`, so the caller's strict
    /// check declines — consistent with the shared shell's
    /// borrow-conflict → defined-rejection policy.
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        self.state
            .try_borrow()
            .map_or(LifecycleState::Failed, |state| state.lifecycle.state())
    }
}

/// Reads `IParameterChanges` into `events`: one [`ParamEvent`] per
/// queue point whose parameter id equals `wanted`, out-of-block
/// offsets and non-finite values dropped. The caller sorts the
/// scratch once after the collector ran — param events are the only
/// event source in this slice. The scan is bounded: at most
/// `MAX_BLOCK_EVENTS` queues and `MAX_BLOCK_EVENTS` points per queue
/// are visited, the scan stops as soon as the bounded scratch fills,
/// and the *total* inspection work is capped by
/// [`MAX_PARAM_SCAN_OPERATIONS`] (queue visits + point reads
/// combined), so a hostile host cannot multiply inspections to
/// `queues × points` while few events are accepted.
///
/// On budget exhaustion the scan stops deterministically — events
/// already collected are retained and processed (the
/// earliest-scanned events survive; the rest are dropped).
///
/// # Safety
///
/// `changes` may be null; otherwise it must point to a valid
/// `IParameterChanges` object for the duration of the call (the
/// VST3 `process` contract guarantees it for
/// `data.inputParameterChanges`). Returned queues are borrowed
/// (`ComRef`) and never stored.
#[expect(unsafe_code)]
pub(crate) unsafe fn collect_param_events(
    changes: *mut IParameterChanges,
    frames: usize,
    wanted: ParamId,
    events: &mut EventScratch,
) {
    {
        events.clear();
        // SAFETY: caller guarantees `changes` is null or a valid
        // `IParameterChanges` for this call — `ComRef` borrows without
        // touching the refcount.
        let Some(changes) = (unsafe { ComRef::from_raw(changes) }) else {
            return;
        };
        // SAFETY: `changes` is a live `IParameterChanges` for the call.
        let count = usize::try_from(unsafe { changes.getParameterCount() }.max(0)).unwrap_or(0);
        // Total work budget: each queue visit and each `getPoint`
        // inspection consumes one unit — the scan cost itself is
        // bounded independent of how many events are accepted.
        let mut scan_budget = MAX_PARAM_SCAN_OPERATIONS;
        // A host may declare arbitrary queue/point counts; the scan
        // stays bounded — at most `MAX_BLOCK_EVENTS` queues and
        // `MAX_BLOCK_EVENTS` points per queue, all within the total
        // budget above.
        'queues: for index in 0..count.min(MAX_BLOCK_EVENTS) {
            if scan_budget == 0 {
                break;
            }
            scan_budget -= 1;
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
            if param_id != wanted {
                continue;
            }
            // SAFETY: `queue` is a live `IParamValueQueue` for the call.
            let points = usize::try_from(unsafe { queue.getPointCount() }.max(0)).unwrap_or(0);
            for point in 0..points.min(MAX_BLOCK_EVENTS) {
                if scan_budget == 0 {
                    break 'queues;
                }
                scan_budget -= 1;
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
    }
}

/// Collects one block's neutral event set into `scratch` — parameter
/// queue points always, `IEventList` MIDI only when `midi` is set
/// (the instrument surface) — then sorts and validates into an
/// [`EventList`] borrowing `scratch`. The returned list keeps the
/// `scratch` borrow alive; callers must not touch `scratch` until the
/// list is dropped.
///
/// # Safety
///
/// `param_changes` and `input_events` must each be null or a valid
/// object for the duration of the call — the VST3 `process` contract
/// guarantees it for `data.inputParameterChanges`/`data.inputEvents`.
#[expect(unsafe_code)]
pub(crate) unsafe fn collect_block_events(
    scratch: &mut EventScratch,
    param_id: ParamId,
    param_changes: *mut IParameterChanges,
    input_events: *mut IEventList,
    midi: bool,
    frames: usize,
) -> Result<EventList<'_>, ()> {
    // SAFETY: caller upholds the `process`-contract validity of both
    // pointers for this call.
    unsafe {
        collect_param_events(param_changes, frames, param_id, scratch);
        if midi {
            crate::events::collect_midi_events(input_events, frames, scratch);
        }
    }
    // The scratch is sorted — the merge sort is allocation-free
    // over the pre-reserved buffer.
    scratch.sort();
    let frame_count = u32::try_from(frames).unwrap_or(u32::MAX);
    // The collectors drop out-of-block offsets and non-finite values
    // and the scratch is sorted, so `EventList::new` cannot reject;
    // a failure means that invariant broke.
    EventList::new(scratch.as_slice(), frame_count).map_err(|_| ())
}

/// The block's neutral [`TransportInfo`] — only the instrument
/// surface reads the VST3 `ProcessContext`; every other class
/// reports `None`.
///
/// # Safety
///
/// `process_context` must be null or a valid `ProcessContext` for
/// this call (the VST3 `process` contract).
#[expect(unsafe_code)]
fn block_transport(
    surface: InputSurface,
    process_context: *mut VstProcessContext,
    sample_rate: u32,
) -> Option<TransportInfo> {
    if surface != InputSurface::MidiTransport {
        return None;
    }
    // SAFETY: caller upholds the `process`-contract validity of the
    // pointer for this call.
    unsafe { crate::events::transport_from_context(process_context, sample_rate) }
}

impl<P: PluginProcessor> IPluginBaseTrait for TestComponent<P> {
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

    /// `terminate` — legal only from `Initialized`/`Created` (and
    /// `Failed`), i.e. after `Deactivate`; a still-activated
    /// terminate is rejected. Unlike the neutral `Destroy` event —
    /// which is terminal because CLAP `destroy` is — VST3 `terminate`
    /// is *non-terminal*: only COM `release` destroys the instance,
    /// and the SDK's terminf suite re-runs `initialize` on the same
    /// object. A legal terminate therefore returns the lifecycle to
    /// `Created` and drops the negotiated processing config.
    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        match state.lifecycle.transition(LifecycleEvent::Destroy) {
            Ok(_) => {
                state.lifecycle = Lifecycle::new();
                state.config = None;
                kResultOk
            }
            Err(_) => kResultFalse,
        }
    }
}

impl<P: PluginProcessor> IComponentTrait for TestComponent<P> {
    /// Reports the distributed controller class — the host
    /// instantiates `TestGainController` by this CID.
    #[expect(unsafe_code)]
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        if class_id.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        unsafe { *class_id = self.controller_cid };
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    /// One audio input bus, one audio output bus, no event buses.
    #[expect(unsafe_code)]
    unsafe fn getBusCount(&self, media_type: MediaType, dir: BusDirection) -> i32 {
        let is_audio = media_type == sdk_enum_i32(MediaTypes_::kAudio);
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
        i32::from(is_audio && is_known_dir)
    }

    /// Describes the single `kMain` bus on each side — stereo by
    /// default; `channelCount` tracks the negotiated arrangement.
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
        let is_audio = media_type == sdk_enum_i32(MediaTypes_::kAudio);
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
        if !is_audio || !is_known_dir || index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null; only
        // fields of `BusInfo` are written.
        let bus = unsafe { &mut *bus };
        bus.mediaType = sdk_enum_i32(MediaTypes_::kAudio);
        bus.direction = dir;
        // `channelCount` tracks the negotiated arrangement — the SDK
        // validator cross-checks it against `getBusArrangement`
        // (SpeakerArrangementTest); stereo is the pre-negotiation
        // default, matching `bus_channels`' initial value.
        let channels = self
            .state
            .try_borrow()
            .map_or(CHANNELS_PER_BUS, |state| state.bus_channels);
        #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            bus.channelCount = channels as i32;
        }
        copy_wstring(
            if dir == sdk_enum_i32(BusDirections_::kInput) {
                "Input"
            } else {
                "Output"
            },
            &mut bus.name,
        );
        bus.busType = sdk_enum_i32(BusTypes_::kMain);
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

    /// The stateful class's real state verbs — the component-payload
    /// blob round-trips through the [`ComponentStateSurface`]. Every
    /// other class keeps the honest `kNotImplemented`: a silent
    /// `kResultOk` would lose the host's state.
    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        let Some(surface) = &self.state_surface else {
            return kNotImplemented;
        };
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract (null →
        // `kInvalidArgument` inside the helper).
        let bytes = match unsafe { stream_read_all(state) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        if surface.set_component_state(&bytes) {
            kResultOk
        } else {
            kResultFalse
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        let Some(surface) = &self.state_surface else {
            return kNotImplemented;
        };
        let bytes = surface.component_state_blob();
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        unsafe { stream_write_all(state, &bytes) }
    }
}

impl<P: PluginProcessor> IAudioProcessorTrait for TestComponent<P> {
    /// Accepts matching mono or stereo arrangements on the single bus
    /// pair — the declared stereo is the default; mono is a legal
    /// negotiation for the channel-agnostic gain law.
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
        let channels = if inputs == SpeakerArr::kStereo && outputs == SpeakerArr::kStereo {
            CHANNELS_PER_BUS
        } else if inputs == SpeakerArr::kMono && outputs == SpeakerArr::kMono {
            1
        } else {
            return kResultFalse;
        };
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        state.bus_channels = channels;
        kResultTrue
    }

    /// Reports the negotiated arrangement (stereo until a mono
    /// `setBusArrangements` is accepted).
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
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
        if !is_known_dir || index != 0 {
            return kInvalidArgument;
        }
        let channels = self
            .state
            .try_borrow()
            .map_or(CHANNELS_PER_BUS, |state| state.bus_channels);
        // SAFETY: caller-provided out-param, checked non-null.
        unsafe {
            *arr = if channels == 1 {
                SpeakerArr::kMono
            } else {
                SpeakerArr::kStereo
            };
        }
        kResultOk
    }

    /// f32 only: `kSample64` is declined here *and* re-checked at the
    /// `process` boundary (the spec's "never reinterpret f64 as f32"
    /// rule — 64-bit input is rejected, never reinterpreted).
    #[expect(unsafe_code)]
    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: i32) -> tresult {
        if symbolic_sample_size == sdk_enum_i32(SymbolicSampleSizes_::kSample32) {
            kResultOk
        } else if symbolic_sample_size == sdk_enum_i32(SymbolicSampleSizes_::kSample64) {
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
    /// Legal in the disabled states — `Initialized` (the SDK
    /// documents `setupProcessing` as "[UI-thread & (Initialized |
    /// Connected)]" and hosts may call it before `setActive(true)`)
    /// and `Activated` (post-activation re-negotiation); rejected
    /// while `Processing`, before `initialize`, and after `terminate`.
    /// A `kSample64` request is rejected.
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
        if !matches!(
            state.lifecycle.state(),
            LifecycleState::Initialized | LifecycleState::Activated
        ) {
            return kResultFalse;
        }
        if setup.symbolicSampleSize != sdk_enum_i32(SymbolicSampleSizes_::kSample32) {
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
        // disjoint borrows (`event_scratch` borrows while
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
        if data.symbolicSampleSize != sdk_enum_i32(SymbolicSampleSizes_::kSample32) {
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

        let Ok(events) = (unsafe {
            collect_block_events(
                &mut state.event_scratch,
                state.param_id,
                data.inputParameterChanges,
                data.inputEvents,
                self.input_surface == InputSurface::MidiTransport,
                frames,
            )
        }) else {
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

        let rate = state.config.map_or(0, |config| config.sample_rate().get());
        let transport = block_transport(self.input_surface, data.processContext, rate);

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
            let in_channels = usize::try_from(in_bus.numChannels).unwrap_or(usize::MAX);
            let out_channels = usize::try_from(out_bus.numChannels).unwrap_or(usize::MAX);
            // 1..=CHANNELS_PER_BUS channels per bus — the gain law is
            // per-channel; zero or oversized counts are malformed.
            if in_channels == 0
                || in_channels > CHANNELS_PER_BUS
                || out_channels == 0
                || out_channels > CHANNELS_PER_BUS
            {
                return kResultFalse;
            }
            // SAFETY: `channelBuffers32` is the right union member
            // because `symbolicSampleSize == kSample32` was checked
            // above; each bus exposes `numChannels` entries per
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
            // SAFETY: non-null pointer lists with `in_channels`/
            // `out_channels` entries each (checked above) — the
            // contract guarantees `numChannels` channel pointers per
            // bus for this call.
            let in_ptrs = unsafe { slice::from_raw_parts(in_list, in_channels) };
            let out_ptrs = unsafe { slice::from_raw_parts(out_list, out_channels) };
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

impl<P: PluginProcessor> IProcessContextRequirementsTrait for TestComponent<P> {
    /// `ParamsOnly` classes consume no conditionally-valid
    /// process-context fields — `0`. `test_instrument` requests the
    /// musical-time fields its transport click oracle reads:
    /// `kProjectTimeMusicValid`, `kTempoValid`, `kTimeSigValid` and
    /// `kBarPositionValid` (`kPlaying`/`projectTimeSamples` are
    /// unconditionally valid and need no flag).
    #[expect(unsafe_code)]
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        use ProcessContext_::StatesAndFlags_ as F;
        match self.input_surface {
            InputSurface::ParamsOnly => 0,
            InputSurface::MidiTransport => {
                F::kProjectTimeMusicValid as u32
                    | F::kTempoValid as u32
                    | F::kTimeSigValid as u32
                    | F::kBarPositionValid as u32
            }
        }
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
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
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

    /// A `ProcessData` with no buses — the event-only (zero-frame
    /// flush) shape.
    fn event_only_data(changes: *mut IParameterChanges, frames: i32) -> ProcessData {
        ProcessData {
            processMode: 0,
            symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
            numSamples: frames,
            numInputs: 0,
            numOutputs: 0,
            inputs: std::ptr::null_mut(),
            outputs: std::ptr::null_mut(),
            inputParameterChanges: changes,
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: std::ptr::null_mut(),
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
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
                maxSamplesPerBlock: max_frames,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
        }
    }

    /// Builds a `TestChanges` COM pointer from `(param_id, points)`
    /// queues — the host→plugin parameter event source.
    fn changes_ptr(queues: Vec<(u32, Vec<(i32, f64)>)>) -> ComPtr<IParameterChanges> {
        let queues = queues
            .into_iter()
            .map(|(id, points)| {
                ComWrapper::new(TestQueue { id, points })
                    .to_com_ptr::<IParamValueQueue>()
                    .unwrap()
            })
            .collect();
        ComWrapper::new(TestChanges { queues })
            .to_com_ptr::<IParameterChanges>()
            .unwrap()
    }

    /// RT contract exercise: the `process` path over validated host
    /// buffers — the `assert_no_alloc` window (Issue #22 lane) proves
    /// the vtable entry performs no (de)allocation; activation and
    /// teardown calls stay outside the measurement.
    #[expect(unsafe_code)]
    #[test]
    fn rt_process_copies_gain_zero_copy() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);

        // SAFETY: `data` + bus/channel pointers valid for the call.
        let result =
            assert_no_alloc::assert_no_alloc(|| unsafe { processor.process(&raw mut data) });
        assert_eq!(result, kResultOk);
        // SAFETY: live COM objects.
        unsafe {
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
        // Unity gain pass-through.
        assert_eq!(output[0], vec![0.5_f32; 64]);
        assert_eq!(output[1], vec![1.0_f32; 64]);
    }

    /// Sample-accurate automation through the real ABI: a normalized
    /// `0.75` point (→ plain `1.5`) at offset 8 of a 16-frame block —
    /// frames 0-7 keep the default `1.0`, frames 8-15 use `1.5`.
    #[expect(unsafe_code)]
    #[test]
    fn param_event_applies_sample_accurately() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let changes = changes_ptr(vec![(0, vec![(8, 0.75)])]);
        let mut input = [vec![1.0_f32; 16], vec![2.0; 16]];
        let mut output = [vec![0.0_f32; 16], vec![0.0; 16]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(16);
        data.inputParameterChanges = changes.as_ptr();

        // SAFETY: `data`, the bus pointers and `changes` are all valid
        // for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        let mut expected0 = vec![1.0_f32; 8];
        expected0.extend(vec![1.5_f32; 8]);
        let mut expected1 = vec![2.0_f32; 8];
        expected1.extend(vec![3.0_f32; 8]);
        assert_eq!(output[0], expected0);
        assert_eq!(output[1], expected1);
    }

    /// Same-offset events apply in enumeration order — last wins.
    #[expect(unsafe_code)]
    #[test]
    fn same_offset_events_last_wins() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        // normalized 0.25 (→0.5) then 1.0 (→2.0), both at offset 4.
        let changes = changes_ptr(vec![(0, vec![(4, 0.25), (4, 1.0)])]);
        let mut input = [vec![1.0_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(8);
        data.inputParameterChanges = changes.as_ptr();

        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        // Mono block (1 channel) is legal — the law is per-channel.
        assert_eq!(output[0], vec![1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0]);
    }

    /// A zero-frame block flushes offset-0 events without touching
    /// buffers; the new gain persists into the next block.
    #[expect(unsafe_code)]
    #[test]
    fn zero_frame_block_flushes_events() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let changes = changes_ptr(vec![(0, vec![(0, 0.0)])]); // → plain 0.0
        let mut data = event_only_data(changes.as_ptr(), 0);
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }

        let mut input = [vec![1.0_f32; 8], vec![1.0; 8]];
        let mut output = [vec![9.0_f32; 8], vec![9.0; 8]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(8);
        // SAFETY: valid `ProcessData`.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        assert_eq!(output[0], vec![0.0_f32; 8]);
        assert_eq!(output[1], vec![0.0_f32; 8]);
    }

    /// The gain persists across blocks — an event in block 1 still
    /// applies in block 2 with no new events.
    #[expect(unsafe_code)]
    #[test]
    fn gain_persists_across_blocks() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let changes = changes_ptr(vec![(0, vec![(0, 0.75)])]); // → 1.5
        let mut input = [vec![1.0_f32; 4], vec![1.0; 4]];
        let mut output = [vec![0.0_f32; 4], vec![0.0; 4]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(4);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        assert_eq!(output[0], vec![1.5_f32; 4]);

        output[0].fill(0.0);
        output[1].fill(0.0);
        let mut data = block.data(4);
        // SAFETY: valid `ProcessData`.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
        }
        assert_eq!(output[0], vec![1.5_f32; 4]);
        assert_eq!(output[1], vec![1.5_f32; 4]);
    }

    /// The mono arrangement negotiation is accepted and a mono block
    /// processes through the same law.
    #[expect(unsafe_code)]
    #[test]
    fn mono_arrangement_and_block_process() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            let mut in_arr = SpeakerArr::kMono;
            let mut out_arr = SpeakerArr::kMono;
            assert_eq!(
                processor.setBusArrangements(&raw mut in_arr, 1, &raw mut out_arr, 1),
                kResultTrue
            );
            let mut reported = SpeakerArr::kStereo;
            assert_eq!(
                processor.getBusArrangement(
                    sdk_enum_i32(BusDirections_::kInput),
                    0,
                    &raw mut reported
                ),
                kResultOk
            );
            assert_eq!(reported, SpeakerArr::kMono);

            // `BusInfo::channelCount` must track the negotiated
            // arrangement — the SDK validator cross-checks it
            // against `getBusArrangement` (SpeakerArrangementTest).
            let mut bus = BusInfo {
                mediaType: 0,
                direction: 0,
                channelCount: 0,
                name: [0; 128],
                busType: 0,
                flags: 0,
            };
            assert_eq!(
                component.getBusInfo(sdk_enum_i32(MediaTypes_::kAudio), 0, 0, &raw mut bus),
                kResultOk
            );
            assert_eq!(bus.channelCount, 1);

            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
                maxSamplesPerBlock: 32,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
        }

        let mut input = [vec![1.0_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(8);
        // SAFETY: valid `ProcessData`.
        unsafe {
            assert_eq!(processor.process(&raw mut data), kResultOk);
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
        assert_eq!(output[0], vec![1.0_f32; 8]);
    }

    /// Mismatched or unsupported arrangements are defined rejects.
    #[expect(unsafe_code)]
    #[test]
    fn bus_arrangements_reject_mismatches() {
        let component = TestGainComponent::new();
        let mut stereo = SpeakerArr::kStereo;
        let mut mono = SpeakerArr::kMono;
        // SAFETY: valid arrangement out-params.
        unsafe {
            // in=stereo, out=mono → reject.
            assert_eq!(
                component.setBusArrangements(&raw mut stereo, 1, &raw mut mono, 1),
                kResultFalse
            );
            // 5.0 surround input → reject.
            let mut surround = SpeakerArr::k50;
            assert_eq!(
                component.setBusArrangements(&raw mut surround, 1, &raw mut surround, 1),
                kResultFalse
            );
        }
    }

    /// `process` outside `Processing` is a defined reject; protocol
    /// violations never panic across the FFI.
    #[expect(unsafe_code)]
    #[test]
    fn process_rejected_outside_processing_state() {
        let component = TestGainComponent::new();
        let mut data = event_only_data(std::ptr::null_mut(), 64);
        // SAFETY: `data` is a valid `ProcessData` for the call.
        unsafe {
            assert_eq!(component.process(&raw mut data), kResultFalse);
        }
    }

    #[expect(unsafe_code)]
    #[test]
    fn protocol_violations_are_defined_rejects() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
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
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample64),
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

    /// VST3 `terminate` is non-terminal: the SDK's terminf suite
    /// re-runs `initialize` on the same instance and expects
    /// `kResultTrue` — only COM `release` destroys the object, so the
    /// component returns to `Created` and the full handshake works
    /// again.
    #[expect(unsafe_code)]
    #[test]
    fn terminate_allows_reinitialize() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
            // The terminf scenario: initialize must succeed again on
            // the same instance, and the component must be fully
            // usable — setupProcessing included.
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
                maxSamplesPerBlock: 64,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
    }

    /// `setupProcessing` is legal in the disabled state — the SDK
    /// documents it as "[UI-thread & (Initialized | Connected)]"
    /// and the validator's Suspend/Resume suite calls it *before*
    /// `setActive(true)`. It stays rejected before `initialize`.
    #[expect(unsafe_code)]
    #[test]
    fn setup_processing_accepted_while_initialized() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
                maxSamplesPerBlock: 64,
                sampleRate: 48_000.0,
            };
            // Created (never initialized) is still a defined reject.
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultFalse);

            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            // Initialized — the SDK's documented disable-state call.
            assert_eq!(processor.setupProcessing(&raw mut setup), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
            assert_eq!(processor.setProcessing(0), kResultOk);
            assert_eq!(component.setActive(0), kResultOk);
            assert_eq!(component.terminate(), kResultOk);
        }
    }

    /// `maxSamplesPerBlock` beyond `MAX_NEGOTIATED_FRAMES` is a
    /// defined reject — the bounce scratch must stay bounded.
    #[expect(unsafe_code)]
    #[test]
    fn setup_processing_rejects_excessive_max_frames() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe {
            assert_eq!(component.initialize(std::ptr::null_mut()), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            let mut setup = ProcessSetup {
                processMode: 0,
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
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

    /// In-place aliasing (output bus == input bus) bounces through
    /// scratch and writes back deterministically.
    #[expect(unsafe_code)]
    #[test]
    fn in_place_aliased_buffers_bounce_and_write_back() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
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
        let wrapper = ComWrapper::new(TestGainComponent::new());
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
        let result =
            assert_no_alloc::assert_no_alloc(|| unsafe { processor.process(&raw mut data) });
        assert_eq!(result, kResultOk);

        let lying_queue = ComWrapper::new(LyingQueue);
        let queue = lying_queue.to_com_ptr::<IParamValueQueue>().unwrap();
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: vec![queue],
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        let result =
            assert_no_alloc::assert_no_alloc(|| unsafe { processor.process(&raw mut data) });
        assert_eq!(result, kResultOk);
    }

    /// Queues for parameter ids the plugin does not own are skipped
    /// before their points are scanned — foreign events can never
    /// starve the bounded scratch.
    #[expect(unsafe_code)]
    #[test]
    fn foreign_param_queues_never_reach_the_processor() {
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let changes = changes_ptr(vec![(99, vec![(0, 0.25)])]);
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
        let wrapper = ComWrapper::new(TestGainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let changes = changes_ptr(vec![(0, vec![(500, 0.25)])]);
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

    /// State (de)serialization is out of scope — reported honestly as
    /// `kNotImplemented`, never a silent success.
    #[expect(unsafe_code)]
    #[test]
    fn state_stream_reports_not_implemented() {
        let component = TestGainComponent::new();
        // SAFETY: null `IBStream` is unused by the stubs.
        unsafe {
            assert_eq!(component.setState(std::ptr::null_mut()), kNotImplemented);
            assert_eq!(component.getState(std::ptr::null_mut()), kNotImplemented);
        }
    }

    /// Out-of-domain enum values are defined rejects, never wrapped
    /// matches: the `int32` query params compare against the constants
    /// in the `int32` domain.
    #[expect(unsafe_code)]
    #[test]
    fn enum_queries_reject_out_of_range_values() {
        let component = TestGainComponent::new();
        let mut bus = BusInfo {
            mediaType: 0,
            direction: 0,
            channelCount: 0,
            name: [0; 128],
            busType: 0,
            flags: 0,
        };
        // SAFETY: `bus` is a valid `BusInfo` out-param for the calls.
        unsafe {
            assert_eq!(component.getBusCount(-1, -1), 0);
            assert_eq!(component.getBusCount(0, -1), 0);
            assert_eq!(component.getBusCount(-1, 0), 0);
            assert_eq!(component.getBusCount(0, 2), 0);
            // The legal pairs still resolve in the `int32` domain.
            assert_eq!(component.getBusCount(0, 0), 1);
            assert_eq!(component.getBusCount(0, 1), 1);
            assert_eq!(
                component.getBusInfo(-1, 0, 0, &raw mut bus),
                kInvalidArgument
            );
            assert_eq!(
                component.getBusInfo(0, -1, 0, &raw mut bus),
                kInvalidArgument
            );
            assert_eq!(component.canProcessSampleSize(-1), kInvalidArgument);
            assert_eq!(component.canProcessSampleSize(i32::MAX), kInvalidArgument);
            assert_eq!(
                component.canProcessSampleSize(sdk_enum_i32(SymbolicSampleSizes_::kSample32)),
                kResultOk
            );
            assert_eq!(
                component.canProcessSampleSize(sdk_enum_i32(SymbolicSampleSizes_::kSample64)),
                kNotImplemented
            );
        }
    }
}
