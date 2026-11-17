//! `test_stateful`'s preset surface — the real VST3 program-list /
//! unit preset interfaces (Issue #630 preset slice).
//!
//! ## Interface ownership (VST 3.8 `ivstunits.h`)
//!
//! | interface | owner | role |
//! | --------- | ----- | ---- |
//! | `IUnitInfo` | edit controller | enumerates units + program lists, program names/info, unit selection |
//! | `IUnitHandler` / `IUnitHandler2` | edit controller | host → plugin selection notifications |
//! | `IProgramListData` | component | per-program preset data get/set |
//! | `IUnitData` | component | per-unit preset data get/set |
//!
//! The component and controller are separate COM objects the host
//! instantiates independently, so they share **no** mutable state —
//! the preset *data* plane lives entirely on the component (it owns
//! the [`SharedGain`] cell), while the controller exposes the
//! read-mostly enumeration plus selection notifications. This is the
//! standard distributed VST3 split: a host that wants preset bytes
//! calls `IProgramListData`/`IUnitData` on the component; a host that
//! wants to browse presets calls `IUnitInfo` on the controller.
//!
//! ## Preset catalog and data law
//!
//! `test_stateful` ships one program list (`"Factory"`, id `0`) with
//! [`PRESETS`] named programs, each a plain-domain gain. Program and
//! unit *data* payloads reuse the [`crate::stateful`] 16-byte `RFST`
//! blob verbatim — a preset is a serialized gain, so
//! `getProgramData`/`setProgramData`/`getUnitData`/`setUnitData`
//! round-trip through the same deterministic codec the state verbs
//! use. `IUnitData` on the root unit carries the **live** gain (the
//! same blob `IComponent::getState` emits); `IProgramListData`
//! carries each catalog program's stored gain, which `setProgramData`
//! may overwrite per program. Malformed blobs are the defined
//! `kResultFalse` rejection; out-of-range finite gains clamp into the
//! parameter domain per the `ParamInfo` law.
//!
//! ## Declared boundaries
//!
//! * `IUnitInfo::setUnitProgramData` reports `kNotImplemented`: the
//!   controller cannot reach the component's data plane, so preset
//!   data writes are honestly declined here rather than silently
//!   dropped. The supported write path is the component's
//!   `IProgramListData::setProgramData` / `IUnitData::setUnitData`.
//! * `test_stateful` has exactly one unit — the VST3-required root
//!   unit (`kRootUnitId`) — owning the single stereo `kMain` bus
//!   pair. `selectUnit` accepts only `0`; every other id is
//!   `kInvalidArgument`.
//!
//! The newtypes exist because `Class::Interfaces` is a type-level
//! tuple: `test_stateful` is the suite's only preset class, so its
//! component/controller carry the preset interfaces while every
//! other class's QI surface stays exactly what it was. Each newtype
//! delegates the shared `TestComponent`/`ParamController` behavior
//! verbatim and adds only the preset verbs.

use std::ffi::c_char;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use vst3::Class;
use vst3::Steinberg::Vst::{
    BusDirection, BusDirections_, BusInfo, CString, IAudioProcessor, IAudioProcessorTrait,
    IComponent, IComponentHandler, IComponentTrait, IEditController, IEditControllerTrait,
    IProcessContextRequirements, IProcessContextRequirementsTrait, IProgramListData,
    IProgramListDataTrait, IUnitData, IUnitDataTrait, IUnitHandler, IUnitHandler2,
    IUnitHandler2Trait, IUnitHandlerTrait, IUnitInfo, IUnitInfoTrait, IoMode, MediaType,
    MediaTypes_, ParameterInfo, ProcessData, ProcessSetup, ProgramListID, ProgramListInfo,
    RoutingInfo, SpeakerArrangement, String128, TChar, UnitID, UnitInfo, kNoParentUnitId,
    kRootUnitId,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPlugView, IPluginBaseTrait, TBool, TUID, int32, kInvalidArgument,
    kNotImplemented, kResultFalse, kResultOk, kResultTrue, tresult,
};

use crate::component::TestComponent;
use crate::controller::ParamController;
use crate::gain::GAIN_PARAM;
use crate::sdk_enum_i32;
use crate::stateful::{
    STATEFUL_CONTROLLER_CID, SharedGain, TestStatefulProcessor, decode_state_blob,
    encode_state_blob, stream_read_all, stream_write_all,
};
use crate::strings::copy_wstring;

// ---------------------------------------------------------------------------
// Preset catalog — the deterministic factory preset pool
// ---------------------------------------------------------------------------

/// One factory preset — a display `name` and its plain-domain gain.
pub(crate) struct PresetDef {
    /// Program/preset display name.
    pub name: &'static str,
    /// The preset's plain gain — serialized as the `f64` field of the
    /// `RFST` state blob.
    pub gain: f64,
}

/// The built-in factory programs — four distinct gains so a host (or
/// test) can tell programs apart by their data bytes and by the
/// audio they produce.
pub(crate) const PRESETS: &[PresetDef] = &[
    PresetDef {
        name: "Default",
        gain: 1.0,
    },
    PresetDef {
        name: "Half",
        gain: 0.5,
    },
    PresetDef {
        name: "Boost",
        gain: 1.5,
    },
    PresetDef {
        name: "Mute",
        gain: 0.0,
    },
];

/// Number of factory programs — `PRESETS.len()` as a const.
const PRESET_COUNT: usize = PRESETS.len();

/// The single program list's id — `0` is the conventional first list.
pub(crate) const PRESET_LIST_ID: ProgramListID = 0;

/// The program list's display name.
const PRESET_LIST_NAME: &str = "Factory";

/// The root unit's display name.
const UNIT_NAME: &str = "Main";

/// Looks up `preset_def(list_id, program_index)` — `None` for any id
/// outside the single `"Factory"` list or out-of-range index.
fn preset_def(list_id: ProgramListID, program_index: int32) -> Option<&'static PresetDef> {
    if list_id != PRESET_LIST_ID {
        return None;
    }
    let index = usize::try_from(program_index).ok()?;
    PRESETS.get(index)
}

// ---------------------------------------------------------------------------
// PresetSurface — the component-side program/unit data store
// ---------------------------------------------------------------------------

/// The component's preset data plane — the [`SharedGain`] cell the
/// live unit data reads/writes plus a per-program gain store seeded
/// from [`PRESETS`]. All fields are atomics: the VST3 state verbs are
/// non-RT but the `&self` COM methods may be reached from any host
/// thread, so the store stays lock-free like `SharedGain` itself.
pub(crate) struct PresetSurface {
    /// The live gain — `IUnitData` on the root unit reads/writes this
    /// same cell `IComponent::getState`/`setState` and the processor
    /// share, so a unit-data restore is observable in audio.
    gain: SharedGain,
    /// Per-program stored gain (`f64` bits), seeded from [`PRESETS`]
    /// and overwritable via `IProgramListData::setProgramData`.
    programs: [AtomicU64; PRESET_COUNT],
}

impl PresetSurface {
    /// A surface bound to `gain` — the same cell the component's
    /// state surface and processor hold — with programs seeded from
    /// the catalog.
    pub(crate) fn new(gain: SharedGain) -> Self {
        Self {
            gain,
            programs: std::array::from_fn(|index| AtomicU64::new(PRESETS[index].gain.to_bits())),
        }
    }

    /// `programDataSupported` — the single list supports program data.
    fn program_supported(list_id: ProgramListID) -> bool {
        list_id == PRESET_LIST_ID
    }

    /// The stored gain of program `index` — caller bounds-checks.
    fn program_gain(&self, index: usize) -> f64 {
        f64::from_bits(self.programs[index].load(Ordering::Acquire))
    }

    /// Stores a decoded plain gain into program `index`, clamped into
    /// the parameter domain (the same law `SharedGain::store` applies).
    fn set_program_gain(&self, index: usize, plain: f64) {
        self.programs[index].store(
            plain
                .clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value)
                .to_bits(),
            Ordering::Release,
        );
    }

    /// Serializes program `index`'s data — the `RFST` blob in the
    /// plain-gain domain.
    fn program_blob(&self, index: usize) -> [u8; crate::stateful::STATE_BLOB_LEN] {
        encode_state_blob(self.program_gain(index))
    }

    /// Decodes and stores program `index`'s data — `false` on a
    /// malformed blob (the defined `kResultFalse` rejection).
    fn set_program_blob(&self, index: usize, bytes: &[u8]) -> bool {
        let Some(plain) = decode_state_blob(bytes) else {
            return false;
        };
        self.set_program_gain(index, plain);
        true
    }

    /// The root unit's data — the live gain blob, identical to what
    /// `IComponent::getState` emits.
    fn unit_blob(&self) -> [u8; crate::stateful::STATE_BLOB_LEN] {
        encode_state_blob(self.gain.load())
    }

    /// Decodes and applies the root unit's data to the live gain —
    /// `false` on a malformed blob.
    fn set_unit_blob(&self, bytes: &[u8]) -> bool {
        let Some(plain) = decode_state_blob(bytes) else {
            return false;
        };
        self.gain.store(plain);
        true
    }
}

// ---------------------------------------------------------------------------
// TestStatefulComponent — the component with the preset data interfaces
// ---------------------------------------------------------------------------

/// `test_stateful`'s component — the shared [`TestComponent`] shell
/// over [`TestStatefulProcessor`] plus the [`PresetSurface`] data
/// plane, exposing `IProgramListData` + `IUnitData` on top of the
/// standard component interfaces. Every other fixture class keeps the
/// 3-interface `TestComponent` surface; only this class carries the
/// preset extensions, so a host QI-ing them on `test_gain` & co.
/// still gets the honest no-interface answer.
pub(crate) struct TestStatefulComponent {
    /// The shared component shell — lifecycle, buses, process path,
    /// and the `IComponent` state verbs over the same [`SharedGain`]
    /// cell `preset` reads/writes.
    inner: TestComponent<TestStatefulProcessor>,
    /// The program/unit preset data plane.
    preset: PresetSurface,
}

impl TestStatefulComponent {
    /// The component class ID — a fresh `uid` for the test suite,
    /// distinct from every other fixture and adapter CID.
    pub(crate) const CID: TUID = vst3::uid(0x8D2F_6B94, 0xA1C5_3E07, 0xF4B8_9D26, 0xC7E3_5A81);

    /// A fresh component — the shared-gain state surface plus the
    /// preset catalog store.
    pub(crate) fn new() -> Self {
        let gain = SharedGain::new(GAIN_PARAM.default_value);
        Self {
            inner: TestComponent::with_state(
                STATEFUL_CONTROLLER_CID,
                GAIN_PARAM.id,
                TestStatefulProcessor::new(gain.clone()),
                gain.clone(),
            ),
            preset: PresetSurface::new(gain),
        }
    }
}

impl Class for TestStatefulComponent {
    type Interfaces = (
        IComponent,
        IAudioProcessor,
        IProcessContextRequirements,
        IProgramListData,
        IUnitData,
    );
}

// -- Delegation: the shared `TestComponent` behavior, verbatim ---------

impl IPluginBaseTrait for TestStatefulComponent {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        unsafe { self.inner.initialize(context) }
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        unsafe { self.inner.terminate() }
    }
}

impl IComponentTrait for TestStatefulComponent {
    #[expect(unsafe_code)]
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        unsafe { self.inner.getControllerClassId(class_id) }
    }

    #[expect(unsafe_code)]
    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        unsafe { self.inner.setIoMode(mode) }
    }

    #[expect(unsafe_code)]
    unsafe fn getBusCount(&self, media_type: MediaType, dir: BusDirection) -> i32 {
        unsafe { self.inner.getBusCount(media_type, dir) }
    }

    #[expect(unsafe_code)]
    unsafe fn getBusInfo(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        unsafe { self.inner.getBusInfo(media_type, dir, index, bus) }
    }

    #[expect(unsafe_code)]
    unsafe fn getRoutingInfo(
        &self,
        in_info: *mut RoutingInfo,
        out_info: *mut RoutingInfo,
    ) -> tresult {
        unsafe { self.inner.getRoutingInfo(in_info, out_info) }
    }

    #[expect(unsafe_code)]
    unsafe fn activateBus(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
        state: TBool,
    ) -> tresult {
        unsafe { self.inner.activateBus(media_type, dir, index, state) }
    }

    #[expect(unsafe_code)]
    unsafe fn setActive(&self, state: TBool) -> tresult {
        unsafe { self.inner.setActive(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        unsafe { self.inner.setState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        unsafe { self.inner.getState(state) }
    }
}

impl IAudioProcessorTrait for TestStatefulComponent {
    #[expect(unsafe_code)]
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        unsafe {
            self.inner
                .setBusArrangements(inputs, num_ins, outputs, num_outs)
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getBusArrangement(
        &self,
        dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        unsafe { self.inner.getBusArrangement(dir, index, arr) }
    }

    #[expect(unsafe_code)]
    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: i32) -> tresult {
        unsafe { self.inner.canProcessSampleSize(symbolic_sample_size) }
    }

    #[expect(unsafe_code)]
    unsafe fn getLatencySamples(&self) -> u32 {
        unsafe { self.inner.getLatencySamples() }
    }

    #[expect(unsafe_code)]
    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        unsafe { self.inner.setupProcessing(setup) }
    }

    #[expect(unsafe_code)]
    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        unsafe { self.inner.setProcessing(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        unsafe { self.inner.process(data) }
    }

    #[expect(unsafe_code)]
    unsafe fn getTailSamples(&self) -> u32 {
        unsafe { self.inner.getTailSamples() }
    }
}

impl IProcessContextRequirementsTrait for TestStatefulComponent {
    #[expect(unsafe_code)]
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        unsafe { self.inner.getProcessContextRequirements() }
    }
}

// -- IProgramListData — per-program preset data -------------------------

impl IProgramListDataTrait for TestStatefulComponent {
    /// `kResultTrue` for the single `"Factory"` list, `kResultFalse`
    /// for any other id — the defined "no data for this list" answer.
    #[expect(unsafe_code)]
    unsafe fn programDataSupported(&self, list_id: ProgramListID) -> tresult {
        if PresetSurface::program_supported(list_id) {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    /// Writes program `program_index`'s stored gain blob — the same
    /// `RFST` format `getState` uses. Out-of-range ids are
    /// `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn getProgramData(
        &self,
        list_id: ProgramListID,
        program_index: int32,
        data: *mut IBStream,
    ) -> tresult {
        let Some(index) = preset_slot(list_id, program_index) else {
            return kInvalidArgument;
        };
        if data.is_null() {
            return kInvalidArgument;
        }
        let bytes = self.preset.program_blob(index);
        // SAFETY: `data` is a valid `IBStream` for the call's duration
        // per the VST3 preset-data contract.
        unsafe { stream_write_all(data, &bytes) }
    }

    /// Reads and stores program `program_index`'s data — a malformed
    /// blob is the defined `kResultFalse` rejection, matching
    /// `IComponent::setState`.
    #[expect(unsafe_code)]
    unsafe fn setProgramData(
        &self,
        list_id: ProgramListID,
        program_index: int32,
        data: *mut IBStream,
    ) -> tresult {
        let Some(index) = preset_slot(list_id, program_index) else {
            return kInvalidArgument;
        };
        if data.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: `data` is a valid `IBStream` for the call's duration.
        let bytes = match unsafe { stream_read_all(data) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        if self.preset.set_program_blob(index, &bytes) {
            kResultOk
        } else {
            kResultFalse
        }
    }
}

/// Bounds-checks `(list_id, program_index)` into a `PRESETS` index —
/// `None` for any out-of-range pair.
fn preset_slot(list_id: ProgramListID, program_index: int32) -> Option<usize> {
    preset_def(list_id, program_index).map(|_| usize::try_from(program_index).unwrap_or(usize::MAX))
}

// -- IUnitData — per-unit preset data ------------------------------------

impl IUnitDataTrait for TestStatefulComponent {
    /// `kResultTrue` for the root unit only — the single-unit plugin
    /// has no other units to carry data.
    #[expect(unsafe_code)]
    unsafe fn unitDataSupported(&self, unit_id: UnitID) -> tresult {
        if unit_id == kRootUnitId {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    /// Writes the root unit's live gain blob — identical bytes to
    /// `IComponent::getState` since both serialize the same
    /// [`SharedGain`] cell.
    #[expect(unsafe_code)]
    unsafe fn getUnitData(&self, unit_id: UnitID, data: *mut IBStream) -> tresult {
        if unit_id != kRootUnitId || data.is_null() {
            return kInvalidArgument;
        }
        let bytes = self.preset.unit_blob();
        // SAFETY: `data` is a valid `IBStream` for the call's duration.
        unsafe { stream_write_all(data, &bytes) }
    }

    /// Reads and applies the root unit's data to the live gain — a
    /// malformed blob is the defined `kResultFalse` rejection.
    #[expect(unsafe_code)]
    unsafe fn setUnitData(&self, unit_id: UnitID, data: *mut IBStream) -> tresult {
        if unit_id != kRootUnitId || data.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: `data` is a valid `IBStream` for the call's duration.
        let bytes = match unsafe { stream_read_all(data) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        if self.preset.set_unit_blob(&bytes) {
            kResultOk
        } else {
            kResultFalse
        }
    }
}

// ---------------------------------------------------------------------------
// TestStatefulController — the controller with the unit/preset surface
// ---------------------------------------------------------------------------

/// `test_stateful`'s edit controller — the shared [`ParamController`]
/// parameter/state surface plus the `IUnitInfo`/`IUnitHandler`/
/// `IUnitHandler2` preset enumeration and notification interfaces.
/// Like the component, this is a distinct `Class` so only
/// `test_stateful`'s controller CID exposes the preset extensions.
pub(crate) struct TestStatefulController {
    /// The shared controller shell — the `Gain` parameter surface and
    /// the live `test_stateful` state verbs.
    inner: ParamController,
    /// Currently selected unit — always the root unit (`0`); kept as
    /// an atomic because the `&self` COM methods may be reached from
    /// any host thread.
    selected_unit: AtomicI32,
    /// Last `notifyProgramListChange` payload — `(list_id, program)`
    /// or `(-1, -1)` before the first notification. Recorded so the
    /// fixture's unit tests can observe the host→plugin signal landed.
    notified_list: AtomicI32,
    /// See [`Self::notified_list`].
    notified_program: AtomicI32,
}

impl TestStatefulController {
    /// A fresh controller — the stateful `ParamController` plus an
    /// empty notification record.
    pub(crate) fn new() -> Self {
        Self {
            inner: ParamController::new_stateful(&GAIN_PARAM, 0),
            selected_unit: AtomicI32::new(kRootUnitId),
            notified_list: AtomicI32::new(-1),
            notified_program: AtomicI32::new(-1),
        }
    }
}

impl Class for TestStatefulController {
    type Interfaces = (IEditController, IUnitInfo, IUnitHandler, IUnitHandler2);
}

// -- Delegation: the shared `ParamController` behavior, verbatim --------

impl IPluginBaseTrait for TestStatefulController {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        unsafe { self.inner.initialize(context) }
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        unsafe { self.inner.terminate() }
    }
}

impl IEditControllerTrait for TestStatefulController {
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        unsafe { self.inner.setComponentState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        unsafe { self.inner.setState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        unsafe { self.inner.getState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        unsafe { self.inner.getParameterCount() }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterInfo(&self, param_index: i32, info: *mut ParameterInfo) -> tresult {
        unsafe { self.inner.getParameterInfo(param_index, info) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        value_normalized: f64,
        string: *mut String128,
    ) -> tresult {
        unsafe {
            self.inner
                .getParamStringByValue(id, value_normalized, string)
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamValueByString(
        &self,
        id: u32,
        string: *mut TChar,
        value_normalized: *mut f64,
    ) -> tresult {
        unsafe {
            self.inner
                .getParamValueByString(id, string, value_normalized)
        }
    }

    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        unsafe { self.inner.normalizedParamToPlain(id, value_normalized) }
    }

    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        unsafe { self.inner.plainParamToNormalized(id, plain_value) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        unsafe { self.inner.getParamNormalized(id) }
    }

    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        unsafe { self.inner.setParamNormalized(id, value) }
    }

    #[expect(unsafe_code)]
    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        unsafe { self.inner.setComponentHandler(handler) }
    }

    #[expect(unsafe_code)]
    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        unsafe { self.inner.createView(name) }
    }
}

// -- IUnitInfo — unit/program-list enumeration ---------------------------

impl IUnitInfoTrait for TestStatefulController {
    /// One unit — the VST3-required root unit.
    #[expect(unsafe_code)]
    unsafe fn getUnitCount(&self) -> int32 {
        1
    }

    /// Fills `info` for the root unit — id `0`, no parent, the single
    /// `"Factory"` program list. Any other index is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn getUnitInfo(&self, unit_index: int32, info: *mut UnitInfo) -> tresult {
        if info.is_null() || unit_index != kRootUnitId {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.id = kRootUnitId;
        info.parentUnitId = kNoParentUnitId;
        copy_wstring(UNIT_NAME, &mut info.name);
        info.programListId = PRESET_LIST_ID;
        kResultOk
    }

    /// One program list — `"Factory"`.
    #[expect(unsafe_code)]
    unsafe fn getProgramListCount(&self) -> int32 {
        1
    }

    /// Fills `info` for the `"Factory"` list — id `0`, `PRESETS.len()`
    /// programs. Any other index is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn getProgramListInfo(&self, list_index: int32, info: *mut ProgramListInfo) -> tresult {
        if info.is_null() || list_index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.id = PRESET_LIST_ID;
        copy_wstring(PRESET_LIST_NAME, &mut info.name);
        info.programCount = i32::try_from(PRESET_COUNT).unwrap_or(i32::MAX);
        kResultOk
    }

    /// The program's display name — `kInvalidArgument` for any
    /// out-of-range `(list_id, program_index)`.
    #[expect(unsafe_code)]
    unsafe fn getProgramName(
        &self,
        list_id: ProgramListID,
        program_index: int32,
        name: *mut String128,
    ) -> tresult {
        if name.is_null() {
            return kInvalidArgument;
        }
        let Some(def) = preset_def(list_id, program_index) else {
            return kInvalidArgument;
        };
        // SAFETY: caller-provided out-param, checked non-null.
        let name = unsafe { &mut *name };
        copy_wstring(def.name, name);
        kResultOk
    }

    /// Program attributes — only `"name"` is defined (the preset's
    /// display name); every other attribute id is the `kResultFalse`
    /// "not present" answer. Out-of-range `(list_id, program_index)`
    /// is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn getProgramInfo(
        &self,
        list_id: ProgramListID,
        program_index: int32,
        attribute_id: CString,
        attribute_value: *mut String128,
    ) -> tresult {
        if attribute_value.is_null() {
            return kInvalidArgument;
        }
        let Some(def) = preset_def(list_id, program_index) else {
            return kInvalidArgument;
        };
        // SAFETY: `attribute_id` is a NUL-terminated `char8` string or
        // null per the VST3 contract; `cstring_eq` bounds the scan.
        if !unsafe { crate::strings::cstring_eq(attribute_id, "name") } {
            return kResultFalse;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let attribute_value = unsafe { &mut *attribute_value };
        copy_wstring(def.name, attribute_value);
        kResultOk
    }

    /// No program pitch names — the defined `kResultFalse`.
    #[expect(unsafe_code)]
    unsafe fn hasProgramPitchNames(
        &self,
        _list_id: ProgramListID,
        _program_index: int32,
    ) -> tresult {
        kResultFalse
    }

    /// No program pitch names exist — `kResultFalse`.
    #[expect(unsafe_code)]
    unsafe fn getProgramPitchName(
        &self,
        _list_id: ProgramListID,
        _program_index: int32,
        _midi_pitch: i16,
        _name: *mut String128,
    ) -> tresult {
        kResultFalse
    }

    /// The selected unit — always the root unit.
    #[expect(unsafe_code)]
    unsafe fn getSelectedUnit(&self) -> UnitID {
        self.selected_unit.load(Ordering::Acquire)
    }

    /// Selects the root unit — the only unit that exists. Any other
    /// id is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn selectUnit(&self, unit_id: UnitID) -> tresult {
        if unit_id != kRootUnitId {
            return kInvalidArgument;
        }
        self.selected_unit.store(kRootUnitId, Ordering::Release);
        kResultOk
    }

    /// Maps the single stereo `kMain` bus pair to the root unit —
    /// audio input/output bus `0`, channel `-1` (all) or `0..2`.
    /// Anything else is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn getUnitByBus(
        &self,
        r#type: MediaType,
        dir: BusDirection,
        bus_index: int32,
        channel: int32,
        unit_id: *mut UnitID,
    ) -> tresult {
        if unit_id.is_null() {
            return kInvalidArgument;
        }
        let is_audio = r#type == sdk_enum_i32(MediaTypes_::kAudio);
        let is_known_dir = dir == sdk_enum_i32(BusDirections_::kInput)
            || dir == sdk_enum_i32(BusDirections_::kOutput);
        let channel_ok = channel == -1 || (0..2).contains(&channel);
        if !is_audio || !is_known_dir || bus_index != 0 || !channel_ok {
            return kInvalidArgument;
        }
        // SAFETY: out-param checked non-null.
        unsafe { *unit_id = kRootUnitId };
        kResultOk
    }

    /// Declared boundary — the controller cannot reach the
    /// component's preset data plane, so preset-data writes through
    /// this verb are honestly `kNotImplemented` rather than silently
    /// dropped. The supported write path is the component's
    /// `IProgramListData::setProgramData` / `IUnitData::setUnitData`.
    #[expect(unsafe_code)]
    unsafe fn setUnitProgramData(
        &self,
        _list_or_unit_id: int32,
        _program_index: int32,
        _data: *mut IBStream,
    ) -> tresult {
        kNotImplemented
    }
}

// -- IUnitHandler / IUnitHandler2 — selection notifications -------------

impl IUnitHandlerTrait for TestStatefulController {
    /// The root unit is the only selection — `kResultOk` for `0`,
    /// `kInvalidArgument` otherwise.
    #[expect(unsafe_code)]
    unsafe fn notifyUnitSelection(&self, unit_id: UnitID) -> tresult {
        if unit_id != kRootUnitId {
            return kInvalidArgument;
        }
        self.selected_unit.store(kRootUnitId, Ordering::Release);
        kResultOk
    }

    /// Records a valid program-list selection — `(0, index)` inside
    /// `PRESETS` is `kResultOk` and lands in the notification record;
    /// anything else is `kInvalidArgument`.
    #[expect(unsafe_code)]
    unsafe fn notifyProgramListChange(
        &self,
        list_id: ProgramListID,
        program_index: int32,
    ) -> tresult {
        if preset_def(list_id, program_index).is_none() {
            return kInvalidArgument;
        }
        self.notified_list.store(list_id, Ordering::Release);
        self.notified_program
            .store(program_index, Ordering::Release);
        kResultOk
    }
}

impl IUnitHandler2Trait for TestStatefulController {
    /// Single bus pair → single unit: nothing to recompute, the
    /// notification is acknowledged.
    #[expect(unsafe_code)]
    unsafe fn notifyUnitByBusChange(&self) -> tresult {
        kResultOk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use vst3::Steinberg::IBStream_::IStreamSeekMode_::{kIBSeekCur, kIBSeekEnd, kIBSeekSet};
    use vst3::Steinberg::IBStreamTrait;
    use vst3::{Class, ComPtr, ComWrapper};

    /// A `Vec`-backed `IBStream` whose bytes stay inspectable through
    /// a shared handle — the preset verbs' read/write oracle. Mirrors
    /// `stateful`'s `TestStream` semantics (EOF-honest reads,
    /// zero-filled seek gaps, `int64` positions) for the preset data
    /// plane.
    #[derive(Default)]
    struct VecStreamState {
        data: Vec<u8>,
        pos: u64,
    }

    struct VecStream {
        state: Arc<Mutex<VecStreamState>>,
    }

    /// A wrapped stream plus the shared handle used to inspect the
    /// bytes the plugin wrote or seeded for reads.
    fn vec_stream(bytes: Vec<u8>) -> (ComWrapper<VecStream>, Arc<Mutex<VecStreamState>>) {
        let state = Arc::new(Mutex::new(VecStreamState {
            data: bytes,
            pos: 0,
        }));
        (
            ComWrapper::new(VecStream {
                state: Arc::clone(&state),
            }),
            state,
        )
    }

    /// The `IBStream` COM pointer of a wrapped [`VecStream`] — the
    /// returned `ComPtr` owns a reference and must outlive the FFI
    /// call it feeds.
    fn ib_stream(stream: &ComWrapper<VecStream>) -> ComPtr<IBStream> {
        stream
            .to_com_ptr::<IBStream>()
            .expect("VecStream must expose IBStream")
    }

    impl Class for VecStream {
        type Interfaces = (IBStream,);
    }

    #[expect(unsafe_code)]
    impl IBStreamTrait for VecStream {
        unsafe fn read(
            &self,
            buffer: *mut std::ffi::c_void,
            num_bytes: i32,
            num_bytes_read: *mut i32,
        ) -> tresult {
            if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                return kInvalidArgument;
            }
            let mut state = self.state.lock().expect("stream mutex");
            let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
            let count = state
                .data
                .len()
                .saturating_sub(start)
                .min(usize::try_from(num_bytes).unwrap_or(0));
            if count > 0 {
                // SAFETY: `buffer` is a valid `num_bytes`-byte
                // destination; `count <= num_bytes` and the source is
                // inside `data`.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        state.data[start..start + count].as_ptr(),
                        buffer.cast::<u8>(),
                        count,
                    );
                }
            }
            state.pos = state
                .pos
                .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
            if !num_bytes_read.is_null() {
                // SAFETY: contract out-ptr; `count <= i32::MAX`.
                unsafe { *num_bytes_read = i32::try_from(count).unwrap_or(i32::MAX) };
            }
            kResultOk
        }

        unsafe fn write(
            &self,
            buffer: *mut std::ffi::c_void,
            num_bytes: i32,
            num_bytes_written: *mut i32,
        ) -> tresult {
            if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                return kInvalidArgument;
            }
            let mut state = self.state.lock().expect("stream mutex");
            let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
            let count = usize::try_from(num_bytes).unwrap_or(0);
            if count > 0 {
                let end = start.saturating_add(count);
                state.data.resize(end, 0);
                // SAFETY: `buffer` points at `num_bytes` readable
                // bytes; the destination was sized for `count`.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buffer.cast::<u8>(),
                        state.data[start..end].as_mut_ptr(),
                        count,
                    );
                }
                state.pos = state
                    .pos
                    .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
            }
            if !num_bytes_written.is_null() {
                // SAFETY: contract out-ptr; `count <= i32::MAX`.
                unsafe { *num_bytes_written = i32::try_from(count).unwrap_or(i32::MAX) };
            }
            kResultOk
        }

        unsafe fn seek(&self, pos: i64, mode: i32, result: *mut i64) -> tresult {
            let mut state = self.state.lock().expect("stream mutex");
            let base = if mode == sdk_enum_i32(kIBSeekSet) {
                0
            } else if mode == sdk_enum_i32(kIBSeekCur) {
                i64::try_from(state.pos).unwrap_or(i64::MAX)
            } else if mode == sdk_enum_i32(kIBSeekEnd) {
                i64::try_from(state.data.len()).unwrap_or(i64::MAX)
            } else {
                return kInvalidArgument;
            };
            let Some(target) = base.checked_add(pos) else {
                return kResultFalse;
            };
            if target < 0 {
                return kResultFalse;
            }
            state.pos = u64::try_from(target).unwrap_or(u64::MAX);
            if !result.is_null() {
                // SAFETY: `result` is the contract's out-ptr.
                unsafe { *result = target };
            }
            kResultOk
        }

        unsafe fn tell(&self, pos: *mut i64) -> tresult {
            let state = self.state.lock().expect("stream mutex");
            if !pos.is_null() {
                // SAFETY: `pos` is the contract's out-ptr.
                unsafe { *pos = i64::try_from(state.pos).unwrap_or(i64::MAX) };
            }
            kResultOk
        }
    }

    /// The component plus its COM interface pointers for the preset
    /// verbs — all kept alive for the test's duration.
    struct Fixture {
        _component: ComWrapper<TestStatefulComponent>,
        component: ComPtr<IComponent>,
        program_data: ComPtr<IProgramListData>,
        unit_data: ComPtr<IUnitData>,
    }

    fn fixture() -> Fixture {
        let component = ComWrapper::new(TestStatefulComponent::new());
        Fixture {
            component: component
                .to_com_ptr::<IComponent>()
                .expect("component must expose IComponent"),
            program_data: component
                .to_com_ptr::<IProgramListData>()
                .expect("test_stateful must expose IProgramListData"),
            unit_data: component
                .to_com_ptr::<IUnitData>()
                .expect("test_stateful must expose IUnitData"),
            _component: component,
        }
    }

    /// Reads a `String128` back as a `String` up to the NUL.
    fn wstring_text(units: &[u16]) -> String {
        let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
        String::from_utf16(&units[..end]).expect("preset names are ASCII")
    }

    /// The component exposes `IProgramListData`/`IUnitData` through
    /// real COM query-interface — `test_stateful` is the only class
    /// that does, and the supported-verbs answer is list/unit-scoped.
    #[expect(unsafe_code)]
    #[test]
    fn component_exposes_preset_interfaces() {
        let fixture = fixture();
        // SAFETY: valid COM pointers for the call's duration.
        unsafe {
            assert_eq!(
                fixture.program_data.programDataSupported(PRESET_LIST_ID),
                kResultTrue
            );
            assert_eq!(fixture.program_data.programDataSupported(9), kResultFalse);
            assert_eq!(
                fixture.unit_data.unitDataSupported(kRootUnitId),
                kResultTrue
            );
            assert_eq!(fixture.unit_data.unitDataSupported(4), kResultFalse);
        }
    }

    /// `IProgramListData` get/set round-trips the catalog blob
    /// byte-exact; malformed blobs and out-of-range ids are defined
    /// rejections.
    #[expect(unsafe_code)]
    #[test]
    fn program_data_round_trips_and_rejects() {
        let fixture = fixture();

        // getProgramData(0, 1) → the "Half" preset blob (gain 0.5).
        let (out_stream, out_state) = vec_stream(Vec::new());
        let out_ptr = ib_stream(&out_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture
                    .program_data
                    .getProgramData(PRESET_LIST_ID, 1, out_ptr.as_ptr()),
                kResultOk
            );
        }
        assert_eq!(out_state.lock().unwrap().data, encode_state_blob(0.5));

        // setProgramData(0, 1, gain 0.25) → stored; getProgramData reads it back.
        let new_blob = encode_state_blob(0.25);
        let (in_stream, _in_state) = vec_stream(new_blob.to_vec());
        let in_ptr = ib_stream(&in_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture
                    .program_data
                    .setProgramData(PRESET_LIST_ID, 1, in_ptr.as_ptr()),
                kResultOk
            );
        }
        let (readback, readback_state) = vec_stream(Vec::new());
        let readback_ptr = ib_stream(&readback);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture
                    .program_data
                    .getProgramData(PRESET_LIST_ID, 1, readback_ptr.as_ptr()),
                kResultOk
            );
        }
        assert_eq!(readback_state.lock().unwrap().data, new_blob);

        // A malformed blob is `kResultFalse` — the defined rejection.
        let (bad, _bad_state) = vec_stream(vec![0xAA; 16]);
        let bad_ptr = ib_stream(&bad);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture
                    .program_data
                    .setProgramData(PRESET_LIST_ID, 1, bad_ptr.as_ptr()),
                kResultFalse
            );
        }

        // Out-of-range list/index are `kInvalidArgument`.
        let (s, _s_state) = vec_stream(Vec::new());
        let s_ptr = ib_stream(&s);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture
                    .program_data
                    .getProgramData(PRESET_LIST_ID, 99, s_ptr.as_ptr()),
                kInvalidArgument
            );
            assert_eq!(
                fixture.program_data.getProgramData(4, 0, s_ptr.as_ptr()),
                kInvalidArgument
            );
        }
    }

    /// `IUnitData` on the root unit carries the **live** gain — a
    /// `setUnitData` restore is observable through
    /// `IComponent::getState`, proving the shared cell.
    #[expect(unsafe_code)]
    #[test]
    fn unit_data_shares_the_live_gain_cell() {
        let fixture = fixture();
        let blob = encode_state_blob(1.25);

        // Restore gain 1.25 via setUnitData.
        let (in_stream, _in_state) = vec_stream(blob.to_vec());
        let in_ptr = ib_stream(&in_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture.unit_data.setUnitData(kRootUnitId, in_ptr.as_ptr()),
                kResultOk
            );
        }

        // getUnitData reads the same live gain back.
        let (out_stream, out_state) = vec_stream(Vec::new());
        let out_ptr = ib_stream(&out_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture.unit_data.getUnitData(kRootUnitId, out_ptr.as_ptr()),
                kResultOk
            );
        }
        assert_eq!(out_state.lock().unwrap().data, blob);

        // `IComponent::getState` emits the identical bytes — the
        // component state surface and the unit-data plane share the
        // `SharedGain` cell.
        let (state_stream, state_out) = vec_stream(Vec::new());
        let state_ptr = ib_stream(&state_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(fixture.component.getState(state_ptr.as_ptr()), kResultOk);
        }
        assert_eq!(state_out.lock().unwrap().data, blob);

        // A foreign unit id and a malformed blob are defined rejections.
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture.unit_data.setUnitData(5, in_ptr.as_ptr()),
                kInvalidArgument
            );
        }
        let (bad, _bad_state) = vec_stream(vec![0_u8; 8]);
        let bad_ptr = ib_stream(&bad);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(
                fixture.unit_data.setUnitData(kRootUnitId, bad_ptr.as_ptr()),
                kResultFalse
            );
        }
    }

    /// `IUnitInfo` enumerates the single root unit and the `"Factory"`
    /// program list with the catalog's program names — through real
    /// COM dispatch on the controller.
    #[expect(unsafe_code)]
    #[expect(clippy::too_many_lines)]
    #[test]
    fn unit_info_enumerates_root_unit_and_factory_list() {
        let controller = ComWrapper::new(TestStatefulController::new());
        let unit_info = controller
            .to_com_ptr::<IUnitInfo>()
            .expect("test_stateful controller must expose IUnitInfo");
        let unit_handler = controller
            .to_com_ptr::<IUnitHandler>()
            .expect("test_stateful controller must expose IUnitHandler");
        let unit_handler2 = controller
            .to_com_ptr::<IUnitHandler2>()
            .expect("test_stateful controller must expose IUnitHandler2");
        let edit = controller
            .to_com_ptr::<IEditController>()
            .expect("test_stateful controller must expose IEditController");

        // SAFETY: valid COM pointers; out-params are live stack values.
        unsafe {
            assert_eq!(unit_info.getUnitCount(), 1);
            assert_eq!(unit_info.getProgramListCount(), 1);

            let mut unit = UnitInfo {
                id: -9,
                parentUnitId: -9,
                name: [0; 128],
                programListId: -9,
            };
            assert_eq!(unit_info.getUnitInfo(kRootUnitId, &raw mut unit), kResultOk);
            assert_eq!(unit.id, kRootUnitId);
            assert_eq!(unit.parentUnitId, kNoParentUnitId);
            assert_eq!(unit.programListId, PRESET_LIST_ID);
            assert_eq!(wstring_text(&unit.name), UNIT_NAME);
            assert_eq!(unit_info.getUnitInfo(1, &raw mut unit), kInvalidArgument);

            let mut list = ProgramListInfo {
                id: -9,
                name: [0; 128],
                programCount: -9,
            };
            assert_eq!(unit_info.getProgramListInfo(0, &raw mut list), kResultOk);
            assert_eq!(list.id, PRESET_LIST_ID);
            assert_eq!(
                usize::try_from(list.programCount).unwrap_or(usize::MAX),
                PRESET_COUNT
            );
            assert_eq!(wstring_text(&list.name), PRESET_LIST_NAME);
            assert_eq!(
                unit_info.getProgramListInfo(1, &raw mut list),
                kInvalidArgument
            );

            // Program names come from the catalog.
            let mut name: String128 = [0; 128];
            assert_eq!(
                unit_info.getProgramName(PRESET_LIST_ID, 1, &raw mut name),
                kResultOk
            );
            assert_eq!(wstring_text(&name), "Half");
            assert_eq!(
                unit_info.getProgramName(PRESET_LIST_ID, 99, &raw mut name),
                kInvalidArgument
            );
            assert_eq!(
                unit_info.getProgramName(3, 0, &raw mut name),
                kInvalidArgument
            );

            // `"name"` is the only defined program attribute.
            let attr = b"name\0";
            let mut value: String128 = [0; 128];
            assert_eq!(
                unit_info.getProgramInfo(PRESET_LIST_ID, 2, attr.as_ptr().cast(), &raw mut value),
                kResultOk
            );
            assert_eq!(wstring_text(&value), "Boost");
            let other = b"u.categ\0";
            assert_eq!(
                unit_info.getProgramInfo(PRESET_LIST_ID, 0, other.as_ptr().cast(), &raw mut value),
                kResultFalse
            );

            // No pitch names.
            assert_eq!(
                unit_info.hasProgramPitchNames(PRESET_LIST_ID, 0),
                kResultFalse
            );

            // Single-unit selection.
            assert_eq!(unit_info.getSelectedUnit(), kRootUnitId);
            assert_eq!(unit_info.selectUnit(kRootUnitId), kResultOk);
            assert_eq!(unit_info.selectUnit(3), kInvalidArgument);

            // The bus pair maps to the root unit.
            let mut mapped = -9;
            assert_eq!(
                unit_info.getUnitByBus(
                    sdk_enum_i32(MediaTypes_::kAudio),
                    sdk_enum_i32(BusDirections_::kInput),
                    0,
                    -1,
                    &raw mut mapped,
                ),
                kResultOk
            );
            assert_eq!(mapped, kRootUnitId);
            assert_eq!(
                unit_info.getUnitByBus(
                    sdk_enum_i32(MediaTypes_::kAudio),
                    sdk_enum_i32(BusDirections_::kOutput),
                    0,
                    1,
                    &raw mut mapped,
                ),
                kResultOk
            );
            assert_eq!(
                unit_info.getUnitByBus(
                    sdk_enum_i32(MediaTypes_::kEvent),
                    sdk_enum_i32(BusDirections_::kInput),
                    0,
                    0,
                    &raw mut mapped,
                ),
                kInvalidArgument
            );

            // `setUnitProgramData` is the declared `kNotImplemented`
            // boundary — preset writes go through the component.
            assert_eq!(
                unit_info.setUnitProgramData(PRESET_LIST_ID, 0, std::ptr::null_mut()),
                kNotImplemented
            );

            // Selection notifications land and validate.
            assert_eq!(unit_handler.notifyUnitSelection(kRootUnitId), kResultOk);
            assert_eq!(unit_handler.notifyUnitSelection(2), kInvalidArgument);
            assert_eq!(
                unit_handler.notifyProgramListChange(PRESET_LIST_ID, 2),
                kResultOk
            );
            assert_eq!(
                unit_handler.notifyProgramListChange(PRESET_LIST_ID, 40),
                kInvalidArgument
            );
            assert_eq!(unit_handler2.notifyUnitByBusChange(), kResultOk);

            // The parameter surface still delegates — one `Gain` param.
            assert_eq!(edit.getParameterCount(), 1);
        }
    }

    /// The component's `IComponent` state verbs still work through
    /// the newtype's delegation — the preset surface did not disturb
    /// the existing `test_stateful` state contract.
    #[expect(unsafe_code)]
    #[test]
    fn component_state_verbs_still_round_trip() {
        let fixture = fixture();
        let blob = encode_state_blob(0.5);

        // setState restores the plain gain; getState re-emits it.
        let (in_stream, _in_state) = vec_stream(blob.to_vec());
        let in_ptr = ib_stream(&in_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(fixture.component.setState(in_ptr.as_ptr()), kResultOk);
        }
        let (out_stream, out_state) = vec_stream(Vec::new());
        let out_ptr = ib_stream(&out_stream);
        // SAFETY: valid COM pointers and a live `IBStream`.
        unsafe {
            assert_eq!(fixture.component.getState(out_ptr.as_ptr()), kResultOk);
        }
        assert_eq!(out_state.lock().unwrap().data, blob);
    }
}
