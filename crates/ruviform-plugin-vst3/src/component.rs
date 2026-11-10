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

// The generated bindings declare flag enum constants as
// `DefaultEnumType` (`c_int` on Windows, `c_uint` elsewhere) while the
// bitfields they write are `uint32`; the remaining `as` casts below are
// required on Windows and a no-op elsewhere — suppress
// `unnecessary_cast` for this file. Enum constants feeding `int32`
// fields or params go through `crate::sdk_enum_i32` instead.
#![allow(clippy::unnecessary_cast)]

use std::cell::RefCell;
use std::slice;
use std::sync::atomic::{AtomicU32, Ordering};

use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
use ruviform_media_core::{MediaTime, MusicalTimeInfo, PPQN, TimeBase, TransportStatus};
use ruviform_plugin_core::{
    Event, EventKind, EventList, Lifecycle, LifecycleEvent, LifecycleState, ParamEvent, ParamId,
    ParamValue, PluginProcessor, ProcessConfig, ProcessContext, ProcessStatus, TransportInfo,
};
use vst3::Steinberg::Vst::{
    BusDirection, BusDirections_, BusInfo, BusInfo_::BusFlags_, BusTypes_, IAudioProcessor,
    IAudioProcessorTrait, IComponent, IComponentTrait, IEventListTrait, IParamValueQueueTrait,
    IParameterChanges, IParameterChangesTrait, IProcessContextRequirements,
    IProcessContextRequirementsTrait, IoMode, MediaType, MediaTypes_, ProcessContext_, ProcessData,
    ProcessSetup, RoutingInfo, SpeakerArr, SpeakerArrangement, SymbolicSampleSizes_,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPluginBaseTrait, TBool, TUID, kInvalidArgument, kNotImplemented,
    kResultFalse, kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComRef, uid};

use crate::block::{self, ChannelScratch, EventScratch, MAX_BLOCK_EVENTS, PreparedBlock};
use crate::controller::GainController;
use crate::gain::{GAIN_PARAM, GainProcessor};
use crate::midi::{MidiBridgeStats, collect_incoming, outgoing_event, ppq_position};
use crate::sdk_enum_i32;
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
    /// Cumulative `IEventList` → canonical translation counters
    /// (Issue #253) — the observable side of the defined-skip policy.
    midi_stats: MidiBridgeStats,
    /// Plugin→host report drain scratch — the processor's queued
    /// [`Event`] reports are moved here once per block, then written
    /// through `ProcessData.outputParameterChanges`/`outputEvents`.
    /// Capacity reserved at construction; pushes stay inside it.
    report_scratch: Vec<Event>,
    /// Report-translation counters — the observable side of the
    /// plugin→host defined-skip policy (null output channels,
    /// out-of-block offsets, unwritable queues).
    report_stats: MidiBridgeStats,
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
            midi_stats: MidiBridgeStats::new(),
            report_scratch: Vec::with_capacity(MAX_BLOCK_EVENTS),
            report_stats: MidiBridgeStats::new(),
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
    /// non-finite values dropped. The caller sorts the scratch once
    /// after all collectors ran — param and MIDI events interleave
    /// by frame offset in the final [`EventList`]. The scan is
    /// bounded: at most `MAX_BLOCK_EVENTS` queues and `MAX_BLOCK_EVENTS`
    /// points per queue are visited, the scan stops as soon as the
    /// bounded scratch fills, and — crucially — the *total* inspection
    /// work is capped by [`MAX_PARAM_SCAN_OPERATIONS`] (queue visits +
    /// point reads combined), so a hostile host cannot multiply
    /// inspections to `queues × points` while few events are accepted.
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
            if param_id != GAIN_PARAM.id {
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

    /// Maps the optional VST3 `ProcessContext` to the neutral
    /// [`TransportInfo`]: `kPlaying` → `Playing`, otherwise `Stopped`;
    /// `projectTimeSamples` → `MediaTime` at the negotiated rate;
    /// musical fields → [`MusicalTimeInfo`] via
    /// [`musical_from_context`]. Returns `None` when the context is
    /// absent or the rate unset — matching the neutral
    /// `Option<TransportInfo>` contract.
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
        let musical = musical_from_context(ctx);
        TimeBase::sample_rate(sample_rate).ok().map(|base| {
            let info = TransportInfo::new(status, MediaTime::new(ctx.projectTimeSamples, base));
            match musical {
                Some(m) => info.with_musical(m),
                None => info,
            }
        })
    }

    /// Drains [`GainProcessor`]'s queued plugin→host reports into the
    /// `ProcessData` output channels: `EventKind::ParamValue` reports
    /// go through `outputParameterChanges` (`addParameterData` →
    /// `addPoint`), `EventKind::Midi` reports through `outputEvents`
    /// (`addEvent` with the same canonical→VST3 translation the
    /// host→plugin direction uses). A null output channel, an
    /// out-of-block offset, an unwritable queue and unknown future
    /// `EventKind`s are defined skips counted in `report_stats` —
    /// never a block rejection and never an allocation.
    ///
    /// # Safety
    ///
    /// `data` must be the live `ProcessData` of the current `process`
    /// call — its `outputParameterChanges`/`outputEvents` fields are
    /// null or valid `IParameterChanges`/`IEventList` pointers for
    /// the duration of the call per the VST3 contract.
    #[expect(unsafe_code)]
    unsafe fn emit_reports(
        data: &ProcessData,
        frames: usize,
        transport: Option<TransportInfo>,
        state: &mut ComponentState,
    ) {
        state.report_scratch.clear();
        state
            .processor
            .drain_output_reports(&mut state.report_scratch);
        if state.report_scratch.is_empty() {
            return;
        }
        let musical = transport.and_then(TransportInfo::musical);
        let sample_rate = state.sample_rate;
        let frame_bound = u32::try_from(frames).unwrap_or(u32::MAX);
        for report in &state.report_scratch {
            // Out-of-block report offsets are a defined skip — the
            // same drop policy the input collectors apply (never a
            // wrong-frame clamp).
            if report.frame_offset() >= frame_bound {
                state.report_stats.skipped += 1;
                continue;
            }
            match report.kind() {
                EventKind::ParamValue(param) => {
                    // SAFETY: `data.outputParameterChanges` is null or
                    // a valid `IParameterChanges` for this call.
                    let Some(changes) = (unsafe { ComRef::from_raw(data.outputParameterChanges) })
                    else {
                        state.report_stats.skipped += 1;
                        continue;
                    };
                    let mut queue_index = 0_i32;
                    // SAFETY: `changes` is live for the call;
                    // `param.param_id.0` is a valid `ParamID` in-param
                    // and `queue_index` a valid out-param.
                    let queue = unsafe {
                        changes.addParameterData(&raw const param.param_id.0, &raw mut queue_index)
                    };
                    let Some(queue) = (unsafe { ComRef::from_raw(queue) }) else {
                        state.report_stats.skipped += 1;
                        continue;
                    };
                    let offset = i32::try_from(report.frame_offset()).unwrap_or(i32::MAX);
                    let mut point_index = 0_i32;
                    // SAFETY: `queue` is live for the call; `offset`/
                    // `param.value.get()` are in-params and
                    // `point_index` a valid out-param.
                    if unsafe { queue.addPoint(offset, param.value.get(), &raw mut point_index) }
                        == kResultTrue
                    {
                        state.report_stats.translated += 1;
                    } else {
                        state.report_stats.skipped += 1;
                    }
                }
                EventKind::Midi(midi) => {
                    // SAFETY: `data.outputEvents` is null or a valid
                    // `IEventList` for this call.
                    let Some(list) = (unsafe { ComRef::from_raw(data.outputEvents) }) else {
                        state.report_stats.skipped += 1;
                        continue;
                    };
                    let ppq = ppq_position(musical, report.frame_offset(), sample_rate);
                    let Some(mut vst) = outgoing_event(report.frame_offset(), &midi, ppq) else {
                        state.report_stats.skipped += 1;
                        continue;
                    };
                    // SAFETY: `list` is live for the call; `vst` is a
                    // valid `Event` value for the call's duration.
                    if unsafe { list.addEvent(&raw mut vst) } == kResultTrue {
                        state.report_stats.translated += 1;
                    } else {
                        state.report_stats.skipped += 1;
                    }
                }
                // `EventKind` is `#[non_exhaustive]` — a future kind
                // has no VST3 output-channel mapping yet: defined skip.
                _ => state.report_stats.skipped += 1,
            }
        }
        state.report_scratch.clear();
    }
}

/// Converts a VST3 PPQ (`TQuarterNotes`) position to PPQN-960 ticks:
/// `ppq × 960`, rounded to nearest — the inverse of the design's
/// `tick / 960` mapping. Saturates to the `i64` range.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
fn ppq_to_ticks(ppq: f64) -> i64 {
    let ticks = (ppq * 960.0).round();
    // `as` on a finite f64 saturates at i64 bounds.
    #[expect(clippy::cast_possible_truncation)]
    {
        ticks as i64
    }
}

/// Maps a VST3 `ProcessContext`'s musical fields to a neutral
/// [`MusicalTimeInfo`]: `Some` only when both
/// `kProjectTimeMusicValid` and `kTempoValid` are set and `tempo` is a
/// finite positive BPM — no implicit 4/4 or tempo is fabricated.
/// Signature maps only with `kTimeSigValid` (else `0`/`0`), and
/// `bar_start_tick` stays `0` unless `kBarPositionValid` accompanies
/// a valid signature.
///
/// **RT contract:** RT-suitable — pure math, allocation-free.
fn musical_from_context(ctx: &VstProcessContext) -> Option<MusicalTimeInfo> {
    let state = ctx.state;
    if state & ProcessContext_::StatesAndFlags_::kProjectTimeMusicValid as u32 == 0
        || state & ProcessContext_::StatesAndFlags_::kTempoValid as u32 == 0
        || !ctx.tempo.is_finite()
        || ctx.tempo <= 0.0
    {
        return None;
    }
    let bpm_milli = (ctx.tempo * 1_000.0).round().clamp(1.0, 999_999_999.0);
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bpm_milli = bpm_milli as u32;
    let (sig_num, sig_den) = if state & ProcessContext_::StatesAndFlags_::kTimeSigValid as u32 != 0
    {
        match (
            u8::try_from(ctx.timeSigNumerator),
            u8::try_from(ctx.timeSigDenominator),
        ) {
            (Ok(n @ 1..), Ok(d @ 1..)) => (n, d),
            _ => (0, 0),
        }
    } else {
        (0, 0)
    };
    Some(MusicalTimeInfo {
        position_ticks: ppq_to_ticks(ctx.projectTimeMusic),
        bar_start_tick: if sig_num != 0
            && state & ProcessContext_::StatesAndFlags_::kBarPositionValid as u32 != 0
        {
            ppq_to_ticks(ctx.barPositionMusic)
        } else {
            0
        },
        bpm_milli,
        ppqn: PPQN.ticks_per_quarter(),
        sig_num,
        sig_den,
    })
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
        let is_audio = media_type == sdk_enum_i32(MediaTypes_::kAudio);
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
        i32::from(is_audio && is_known_dir)
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
        #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            bus.channelCount = CHANNELS_PER_BUS as i32;
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
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
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

        // Events + transport are processed even on zero-frame or
        // no-audio blocks (the zero-frame flush semantics of
        // `EventList`).
        // SAFETY: `data.inputParameterChanges` is null or a valid
        // `IParameterChanges` for this call.
        // SAFETY: `data.inputEvents` is null or a valid `IEventList`
        // for this call — the VST3 `process` contract.
        unsafe {
            Self::collect_param_events(data.inputParameterChanges, frames, state);
            collect_incoming(
                data.inputEvents,
                frames,
                &mut state.event_scratch,
                &mut state.midi_stats,
            );
        }
        // Param and MIDI events interleave by frame offset — sort
        // once after both collectors ran (the merge sort is
        // allocation-free over the pre-reserved buffer).
        state.event_scratch.sort();
        // SAFETY: `data.processContext` is null or a valid
        // `ProcessContext` for this call.
        let transport = unsafe { Self::transport(data.processContext, state.sample_rate) };
        let frame_count = u32::try_from(frames).unwrap_or(u32::MAX);
        // Both collectors drop out-of-block offsets and the scratch
        // is sorted, so `EventList::new` cannot reject; a failure
        // means that invariant broke — a defined reject, never a
        // silently-empty event set.
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

        // Plugin→host reports: drain the processor's queued reports
        // into the host-supplied output channels — runs on every
        // completed `process` call, including `Sleep`/event-only
        // blocks (reports are produced during the event scan).
        // SAFETY: `data` is the live `ProcessData` of this call; its
        // output pointers are null or valid per the VST3 contract.
        unsafe {
            Self::emit_reports(data, frames, transport, state);
        }

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

    /// RT contract exercise: the `process` path over validated host
    /// buffers — the `assert_no_alloc` window (Issue #22 lane) proves
    /// the vtable entry performs no (de)allocation; activation and
    /// teardown calls stay outside the measurement.
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
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
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
            symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
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
            data.symbolicSampleSize = sdk_enum_i32(SymbolicSampleSizes_::kSample64);
            assert_eq!(processor.process(&raw mut data), kResultFalse);

            data.symbolicSampleSize = sdk_enum_i32(SymbolicSampleSizes_::kSample32);
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
            symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
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

    /// A `IParamValueQueue` that declares `i32::MAX` points, counts
    /// every `getPoint` inspection, and fails them all — measures the
    /// *scan* work itself, which the accepted-event count cannot cap.
    struct CountingQueue {
        point_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Class for CountingQueue {
        type Interfaces = (IParamValueQueue,);
    }

    impl IParamValueQueueTrait for CountingQueue {
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
            self.point_calls.fetch_add(1, Ordering::Relaxed);
            kResultFalse
        }

        #[expect(unsafe_code)]
        unsafe fn addPoint(&self, _offset: i32, _value: f64, _index: *mut i32) -> tresult {
            kNotImplemented
        }
    }

    /// The per-queue/per-point caps alone allow `MAX_BLOCK_EVENTS`²
    /// inspections; the combined budget must bound the scan to
    /// `MAX_PARAM_SCAN_OPERATIONS` queue visits + point reads —
    /// measured, not just argued.
    #[expect(unsafe_code)]
    #[test]
    fn rt_param_scan_total_budget_bounds_worst_case() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let point_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let queue_wrapper = ComWrapper::new(CountingQueue {
            point_calls: point_calls.clone(),
        });
        let queue = queue_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        // More queues than the budget can ever reach — every one
        // declaring `i32::MAX` failing points is the hostile worst
        // case for scan work.
        let changes_wrapper = ComWrapper::new(TestChanges {
            queues: (0..32).map(|_| queue.clone()).collect(),
        });
        let changes = changes_wrapper.to_com_ptr::<IParameterChanges>().unwrap();

        let mut input = [vec![0.5_f32; 64], vec![1.0; 64]];
        let mut output = [vec![0.0_f32; 64], vec![0.0; 64]];
        let mut block = TestBlock::new(&mut input, &mut output);
        let mut data = block.data(64);
        data.inputParameterChanges = changes.as_ptr();
        // SAFETY: valid `ProcessData`; `changes` valid for the call.
        let result =
            assert_no_alloc::assert_no_alloc(|| unsafe { processor.process(&raw mut data) });
        assert_eq!(result, kResultOk);

        let inspected = point_calls.load(Ordering::Relaxed);
        // Point inspections plus the ≤32 queue visits stay within the
        // combined budget — not ~32×1024 ≈ 32k and certainly not ~1M.
        assert!(
            inspected <= MAX_PARAM_SCAN_OPERATIONS,
            "getPoint inspections {inspected} exceeded the total budget"
        );
    }

    /// Budget exhaustion is deterministic — events already collected
    /// are retained and processed; only the un-scanned tail drops.
    #[expect(unsafe_code)]
    #[test]
    fn param_scan_budget_exhaustion_keeps_collected_events() {
        let wrapper = ComWrapper::new(GainComponent::new());
        let component = wrapper.to_com_ptr::<IComponent>().unwrap();
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        // SAFETY: live COM objects.
        unsafe { activate(&component, &processor, 64) };

        let good_wrapper = ComWrapper::new(TestQueue {
            id: GAIN_PARAM.id.0,
            points: vec![(0, 0.5)],
        });
        let good_queue = good_wrapper.to_com_ptr::<IParamValueQueue>().unwrap();
        let lying_queue_wrapper = ComWrapper::new(LyingQueue);
        let lying_queue = lying_queue_wrapper
            .to_com_ptr::<IParamValueQueue>()
            .unwrap();
        let mut queues = vec![good_queue];
        queues.extend((0..32).map(|_| lying_queue.clone()));
        let changes_wrapper = ComWrapper::new(TestChanges { queues });
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
        // The first queue's gain point still reached the processor
        // even though the budget ran out on the hostile tail.
        assert_eq!(output[0], vec![0.25_f32; 64]);
        assert_eq!(output[1], vec![0.5_f32; 64]);
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

    /// Out-of-domain enum values are defined rejects, never wrapped
    /// matches: the `int32` query params compare against the constants
    /// in the `int32` domain, so a negative `MediaType`/`BusDirection`
    /// can never alias `kAudio`/`kInput` the way a sign-losing `as`
    /// cast could.
    #[expect(unsafe_code)]
    #[test]
    fn enum_queries_reject_out_of_range_values() {
        let component = GainComponent::new();
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

    /// A zeroed `ProcessContext` (all-zero is a valid bit pattern for
    /// this C POD) with `state` flags set — the other fields stay at
    /// the VST3 "invalid" defaults unless a test fills them.
    #[expect(unsafe_code)]
    fn process_context(state: u32) -> VstProcessContext {
        // SAFETY: `ProcessContext` is a C POD — all-zero is valid.
        let mut ctx: VstProcessContext = unsafe { core::mem::zeroed() };
        ctx.state = state;
        ctx
    }

    #[test]
    fn musical_from_context_maps_valid_fields() {
        use ProcessContext_::StatesAndFlags_ as F;
        let mut ctx = process_context(
            F::kProjectTimeMusicValid as u32
                | F::kTempoValid as u32
                | F::kTimeSigValid as u32
                | F::kBarPositionValid as u32,
        );
        ctx.projectTimeMusic = 2.5;
        ctx.barPositionMusic = 2.0;
        ctx.tempo = 125.5;
        ctx.timeSigNumerator = 6;
        ctx.timeSigDenominator = 8;

        let musical = musical_from_context(&ctx).unwrap();
        assert_eq!(musical.position_ticks, 2_400);
        assert_eq!(musical.bar_start_tick, 1_920);
        assert_eq!(musical.bpm_milli, 125_500);
        assert_eq!(musical.ppqn, 960);
        assert_eq!((musical.sig_num, musical.sig_den), (6, 8));
    }

    #[test]
    fn musical_from_context_gates_and_unset_forms() {
        use ProcessContext_::StatesAndFlags_ as F;
        const BOTH: u32 = F::kProjectTimeMusicValid as u32 | F::kTempoValid as u32;

        // Missing position or tempo validity → None.
        for state in [F::kProjectTimeMusicValid as u32, F::kTempoValid as u32, 0] {
            let mut ctx = process_context(state);
            ctx.tempo = 120.0;
            assert_eq!(musical_from_context(&ctx), None);
        }

        // Non-positive / non-finite tempo → None.
        for tempo in [0.0, -120.0, f64::NAN, f64::INFINITY] {
            let mut ctx = process_context(BOTH);
            ctx.tempo = tempo;
            assert_eq!(musical_from_context(&ctx), None);
        }

        // Without kTimeSigValid: sig 0/0 and bar_start 0 even when
        // the raw fields carry values and kBarPositionValid is set.
        let mut ctx = process_context(BOTH | F::kBarPositionValid as u32);
        ctx.tempo = 120.0;
        ctx.timeSigNumerator = 4;
        ctx.timeSigDenominator = 4;
        ctx.barPositionMusic = 4.0;
        let musical = musical_from_context(&ctx).unwrap();
        assert_eq!((musical.sig_num, musical.sig_den), (0, 0));
        assert_eq!(musical.bar_start_tick, 0);
        assert_eq!(musical.bpm_milli, 120_000);

        // Signature valid but bar flag absent → bar_start 0.
        let mut ctx = process_context(BOTH | F::kTimeSigValid as u32);
        ctx.tempo = 120.0;
        ctx.timeSigNumerator = 4;
        ctx.timeSigDenominator = 4;
        ctx.barPositionMusic = 4.0;
        let musical = musical_from_context(&ctx).unwrap();
        assert_eq!((musical.sig_num, musical.sig_den), (4, 4));
        assert_eq!(musical.bar_start_tick, 0);
    }

    /// The `transport()` wrapper composes media + musical mapping.
    #[expect(unsafe_code)]
    #[test]
    fn transport_combines_media_and_musical() {
        use ProcessContext_::StatesAndFlags_ as F;
        let mut ctx = process_context(
            F::kPlaying as u32 | F::kProjectTimeMusicValid as u32 | F::kTempoValid as u32,
        );
        ctx.projectTimeSamples = 96_000;
        ctx.projectTimeMusic = 1.0;
        ctx.tempo = 60.0;
        // SAFETY: `ctx` outlives the call.
        let info = unsafe { GainComponent::transport(&raw const ctx, 48_000) }.unwrap();
        assert_eq!(info.status(), TransportStatus::Playing);
        assert_eq!(info.position().value(), 96_000);
        let musical = info.musical().unwrap();
        assert_eq!(musical.position_ticks, 960);
        assert_eq!(musical.bpm_milli, 60_000);
        assert_eq!((musical.sig_num, musical.sig_den), (0, 0));
    }
}
