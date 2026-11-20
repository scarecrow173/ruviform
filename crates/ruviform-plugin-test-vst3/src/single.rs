//! `test_stateful_single` — the suite's VST3 **single-component**
//! fixture (Issue #630 single-component slice).
//!
//! ## The single-component contract
//!
//! VST3 normally ships a distributed component/controller pair: the
//! component reports a *separate* controller CID from
//! `getControllerClassId` and the host instantiates the controller as
//! a second object. A single-component plugin instead reports **its
//! own** CID — the same COM object implements `IComponent`,
//! `IAudioProcessor` and `IEditController`, and a conforming host
//! must detect that and `cast` the component pointer instead of
//! creating a second instance.
//!
//! That detection contract is what this class exists to prove:
//! `TestStatefulSingle` is one object whose `getControllerClassId`
//! writes [`Self::CID`] — its own class ID — so the production
//! bridge's `single_component == true` arm runs against a real
//! binary for the first time. Every other class in the suite is a
//! distributed pair and can never reach that arm.
//!
//! ## Shape
//!
//! The component/audio surface delegates verbatim to the shared
//! [`TestComponent`] shell — constructed with `controller_cid` set
//! to the class's own CID and the [`SharedGain`] state surface — so
//! the bus negotiation, lifecycle, DSP and component-domain state
//! law are identical to `test_stateful` (a wrong answer there is a
//! shared bug, not a fixture quirk). The controller surface is this
//! class's own: pure-domain verbs (parameter info, string format/
//! parse, domain mapping, null `createView`) delegate to an embedded
//! [`ParamController`], while the state and value verbs bind the
//! same [`SharedGain`] cell the processor reads — a single
//! component has one source of truth, not a mirror that can drift.
//!
//! ## Deliberate divergences from the distributed precedent
//!
//! - **`terminate` is strict.** The shared shell accepts `terminate`
//!   from `Created` as well as `Initialized` — the lenient reading
//!   the CLAP-derived neutral lifecycle permits — which makes a
//!   double `terminate` on the shared object silently succeed. A
//!   verification fixture exists to *expose* that bug class, so the
//!   single component declines `terminate` unless the object is
//!   currently `Initialized`. A host that terminates the shared
//!   object twice (the regression the `single_component` arm is
//!   meant to prevent) gets a defined `kResultFalse` on the second
//!   call.
//! - **`setComponentHandler` enforces the once-contract.** The VST3
//!   contract hands the handler exactly once after `initialize`; the
//!   distributed controllers ignore the pointer. This class counts
//!   the calls — recorded for in-crate tests — and declines a second
//!   call, so a bridge that double-installs the handler on the
//!   shared object fails at `init` instead of passing silently.
//!   (The handler itself is not retained: the fixture has no
//!   `beginEdit`/`performEdit` behavior to drive.)
//!
//! What stays identical: the 16-byte RFST state blob law
//! ([`crate::stateful`]), the gain parameter law
//! ([`crate::gain::GAIN_PARAM`]), the DSP law
//! ([`crate::stateful::TestStatefulProcessor`]), and the honest
//! `setComponentState`/`setState`/`getState` surface — on a
//! single-component plugin the production bridge never calls the
//! controller state verbs, but they are real surface with specified
//! semantics, not `kNotImplemented` placeholders.

use std::ffi::c_char;
use std::sync::atomic::{AtomicU32, Ordering};

use ruviform_plugin_core::{LifecycleState, ParamValue};
use vst3::Steinberg::Vst::{
    BusDirection, BusInfo, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler,
    IComponentTrait, IEditController, IEditControllerTrait, IProcessContextRequirements,
    IProcessContextRequirementsTrait, IoMode, MediaType, ParameterInfo, ProcessData, ProcessSetup,
    RoutingInfo, SpeakerArrangement, String128, TChar,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPlugView, IPluginBaseTrait, TBool, TUID, kInvalidArgument, kResultFalse,
    kResultOk, tresult,
};
use vst3::{Class, uid};

use crate::component::TestComponent;
use crate::controller::ParamController;
use crate::gain::GAIN_PARAM;
use crate::stateful::{
    SharedGain, TestStatefulProcessor, decode_state_blob, encode_state_blob, stream_read_all,
    stream_write_all,
};

/// The single-component `test_stateful` class: one COM object
/// implementing `IComponent` + `IAudioProcessor` +
/// `IProcessContextRequirements` + `IEditController`.
///
/// `getControllerClassId` reports [`Self::CID`] — the object's own
/// class — which is the VST3 single-component discriminator the
/// production bridge detects (`component_cid == controller_cid`
/// → `single_component`, controller = `cast::<IEditController>` on
/// the component pointer rather than a second `createInstance`).
pub(crate) struct TestStatefulSingle {
    /// The shared component/processor shell — `controller_cid` is
    /// the class's own CID and the state surface is the same
    /// `SharedGain` cell the controller verbs bind.
    inner: TestComponent<TestStatefulProcessor>,
    /// The one source of truth for the gain: the RT processor's
    /// cell, the component state surface's cell, and the controller
    /// value/state verbs' cell are all this object.
    gain: SharedGain,
    /// The data-driven pure-domain verb surface (parameter info,
    /// string format/parse, normalized↔plain mapping, null
    /// `createView`). Its own `value`/`stateful` fields are never
    /// driven — the single component's `gain` cell is authoritative
    /// for every value- or state-carrying verb.
    domain: ParamController,
    /// `setComponentHandler` call count — the VST3 once-contract a
    /// verification fixture owes the bridge (see the module docs).
    handler_calls: AtomicU32,
}

impl TestStatefulSingle {
    /// The component class ID — `getControllerClassId` writes this
    /// same ID, the single-component discriminator.
    pub(crate) const CID: TUID = uid(0x7A3E_5C91, 0x2D4F_8B36, 0xE9A1_6C42, 0xB5D8_3F07);

    pub(crate) fn new() -> Self {
        let gain = SharedGain::new(GAIN_PARAM.default_value);
        Self {
            inner: TestComponent::with_state(
                Self::CID,
                GAIN_PARAM.id,
                TestStatefulProcessor::new(gain.clone()),
                gain.clone(),
            ),
            gain,
            domain: ParamController::new(&GAIN_PARAM, 0),
            handler_calls: AtomicU32::new(0),
        }
    }
}

impl Class for TestStatefulSingle {
    type Interfaces = (
        IComponent,
        IAudioProcessor,
        IProcessContextRequirements,
        IEditController,
    );
}

impl IPluginBaseTrait for TestStatefulSingle {
    /// `Created → Initialized` — the shared shell's transition. A
    /// double `initialize` (e.g. a bridge that also initializes the
    /// `cast`ed controller pointer) is the shell's defined
    /// `kResultFalse` rejection, which fails host `init` — the
    /// externally observable pin this fixture exists to provide.
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        // SAFETY: `context` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.initialize(context) }
    }

    /// Strict `terminate`: legal only while `Initialized`. The
    /// shared shell also accepts `Created` — the lenient reading —
    /// which would let a double `terminate` on the shared object
    /// succeed silently; the single component declines it instead.
    /// Terminate stays non-terminal (the shell returns the lifecycle
    /// to `Created`), so the legal re-init cycle still runs.
    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        if self.inner.lifecycle_state() != LifecycleState::Initialized {
            return kResultFalse;
        }
        unsafe { self.inner.terminate() }
    }
}

impl IComponentTrait for TestStatefulSingle {
    /// Reports the class's *own* CID — the single-component
    /// discriminator (see the struct docs).
    #[expect(unsafe_code)]
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        // SAFETY: `class_id` passes through to the shared shell's
        // contract verbatim.
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
        // SAFETY: `bus` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.getBusInfo(media_type, dir, index, bus) }
    }

    #[expect(unsafe_code)]
    unsafe fn getRoutingInfo(
        &self,
        in_info: *mut RoutingInfo,
        out_info: *mut RoutingInfo,
    ) -> tresult {
        // SAFETY: `in_info`/`out_info` pass through to the shared
        // shell's contract verbatim.
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

    /// Component-payload state — plain gain domain over the shared
    /// RFST codec, identical to `test_stateful`.
    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: `state` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.setState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: `state` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.getState(state) }
    }
}

impl IAudioProcessorTrait for TestStatefulSingle {
    #[expect(unsafe_code)]
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        // SAFETY: all pointers pass through to the shared shell's
        // contract verbatim.
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
        // SAFETY: `arr` passes through to the shared shell's
        // contract verbatim.
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
        // SAFETY: `setup` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.setupProcessing(setup) }
    }

    #[expect(unsafe_code)]
    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        unsafe { self.inner.setProcessing(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        // SAFETY: `data` passes through to the shared shell's
        // contract verbatim.
        unsafe { self.inner.process(data) }
    }

    #[expect(unsafe_code)]
    unsafe fn getTailSamples(&self) -> u32 {
        unsafe { self.inner.getTailSamples() }
    }
}

impl IProcessContextRequirementsTrait for TestStatefulSingle {
    /// `ParamsOnly` — the single component declares the same
    /// no-conditionally-valid-fields surface as `test_stateful`.
    #[expect(unsafe_code)]
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        unsafe { self.inner.getProcessContextRequirements() }
    }
}

impl IEditControllerTrait for TestStatefulSingle {
    /// The *component* payload (plain gain domain) folded into the
    /// single shared cell — on a single-component plugin this is the
    /// same write `IComponent::setState` performs; the production
    /// bridge never calls it here, but it is real surface.
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        let bytes = match unsafe { stream_read_all(state) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        let Some(plain) = decode_state_blob(&bytes) else {
            return kResultFalse;
        };
        // The same fold the component surface performs: `store`
        // clamps finite out-of-range values into the parameter
        // domain.
        self.gain.store(plain);
        kResultOk
    }

    /// Controller payload — normalized domain over the same shared
    /// cell.
    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        let bytes = match unsafe { stream_read_all(state) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        let Some(normalized) = decode_state_blob(&bytes) else {
            return kResultFalse;
        };
        // `ParamValue::new` clamps finite out-of-range normalized
        // values and rejects non-finite — the codec already rejects
        // non-finite, so this accepts every well-formed blob.
        let Ok(value) = ParamValue::new(normalized) else {
            return kResultFalse;
        };
        self.gain.store(GAIN_PARAM.denormalize(value));
        kResultOk
    }

    /// Controller payload — normalized domain, serialized from the
    /// same shared cell.
    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // `normalize` rejects only non-finite input; the cell holds
        // finite values by construction — the `else` is the defined
        // backstop, matching the distributed controller's law.
        let normalized = GAIN_PARAM
            .normalize(self.gain.load())
            .map_or(0.0, ParamValue::get);
        let bytes = encode_state_blob(normalized);
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        unsafe { stream_write_all(state, &bytes) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        unsafe { self.domain.getParameterCount() }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterInfo(&self, param_index: i32, info: *mut ParameterInfo) -> tresult {
        // SAFETY: `info` passes through to the shared domain
        // surface's contract verbatim.
        unsafe { self.domain.getParameterInfo(param_index, info) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        value_normalized: f64,
        string: *mut String128,
    ) -> tresult {
        // SAFETY: `string` passes through to the shared domain
        // surface's contract verbatim.
        unsafe {
            self.domain
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
        // SAFETY: `string`/`value_normalized` pass through to the
        // shared domain surface's contract verbatim.
        unsafe {
            self.domain
                .getParamValueByString(id, string, value_normalized)
        }
    }

    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        unsafe { self.domain.normalizedParamToPlain(id, value_normalized) }
    }

    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        unsafe { self.domain.plainParamToNormalized(id, plain_value) }
    }

    /// Reads the shared cell — unlike the distributed
    /// `ParamController`'s private mirror, a single component's
    /// value surface cannot drift from the processor.
    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == GAIN_PARAM.id.0 {
            GAIN_PARAM
                .normalize(self.gain.load())
                .map_or(0.0, ParamValue::get)
        } else {
            0.0
        }
    }

    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != GAIN_PARAM.id.0 {
            return kInvalidArgument;
        }
        // The normalized domain contract: a non-finite or
        // out-of-range value is a defined rejection, not a stored
        // NaN — the same law `ParamController` applies.
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return kInvalidArgument;
        }
        let Ok(value) = ParamValue::new(value) else {
            return kInvalidArgument;
        };
        self.gain.store(GAIN_PARAM.denormalize(value));
        kResultOk
    }

    /// The VST3 once-contract: the host hands the component handler
    /// exactly once after `initialize`. The call is counted (see
    /// the module docs); a second call is the defined rejection a
    /// verification fixture owes the bridge — a double-install on
    /// the shared object fails at `init` instead of passing
    /// silently.
    #[expect(unsafe_code)]
    unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
        if self.handler_calls.fetch_add(1, Ordering::AcqRel) == 0 {
            kResultOk
        } else {
            kResultFalse
        }
    }

    /// No GUI — `createView` returns null per the VST3 contract.
    #[expect(unsafe_code)]
    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        unsafe { self.domain.createView(name) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    use ruviform_plugin_core::ParamId;
    use vst3::Steinberg::kResultOk;

    use crate::stateful::test_stream::{TestStream, ib_stream};
    use vst3::ComWrapper;

    /// The single-component discriminator: `getControllerClassId`
    /// must write the object's *own* CID — the equality the
    /// production bridge's `single_component` arm detects.
    #[expect(unsafe_code)]
    #[test]
    fn reports_own_controller_class_id() {
        let single = TestStatefulSingle::new();
        let mut cid = [0_i8; 16];
        assert_eq!(
            unsafe { IComponentTrait::getControllerClassId(&single, &raw mut cid) },
            kResultOk
        );
        assert_eq!(cid, TestStatefulSingle::CID);
    }

    /// Strict `terminate`: the shared object declines termination
    /// unless it is currently `Initialized` — a bridge that calls
    /// `controller.terminate` *and* `component.terminate` on the
    /// same object (the double-terminate bug class) gets a defined
    /// rejection on the second call. Terminate stays non-terminal:
    /// the re-init cycle still runs.
    #[expect(unsafe_code)]
    #[test]
    fn terminate_is_strict_to_initialized() {
        let single = TestStatefulSingle::new();
        assert_eq!(unsafe { single.terminate() }, kResultFalse);
        assert_eq!(unsafe { single.initialize(ptr::null_mut()) }, kResultOk);
        assert_eq!(unsafe { single.terminate() }, kResultOk);
        // The second terminate — the double-terminate a buggy bridge
        // would commit — is the defined rejection.
        assert_eq!(unsafe { single.terminate() }, kResultFalse);
        // Re-init still works: VST3 terminate is non-terminal.
        assert_eq!(unsafe { single.initialize(ptr::null_mut()) }, kResultOk);
        assert_eq!(unsafe { single.terminate() }, kResultOk);
    }

    /// A second `initialize` — what a bridge calling
    /// `controller.initialize` on the shared object would commit —
    /// is the shell's defined `IllegalTransition` rejection.
    #[expect(unsafe_code)]
    #[test]
    fn double_initialize_is_rejected() {
        let single = TestStatefulSingle::new();
        assert_eq!(unsafe { single.initialize(ptr::null_mut()) }, kResultOk);
        assert_eq!(unsafe { single.initialize(ptr::null_mut()) }, kResultFalse);
    }

    /// One cell, both domains: a controller-domain normalized write
    /// is the same value the component-domain state verb
    /// serializes, and vice versa — no mirror to drift.
    #[expect(unsafe_code)]
    #[test]
    fn value_and_state_verbs_share_one_cell() {
        let single = TestStatefulSingle::new();
        // Normalized write (0.75 → plain 1.5) → component payload.
        assert_eq!(unsafe { single.setParamNormalized(0, 0.75) }, kResultOk);
        let out = ComWrapper::new(TestStream::new());
        assert_eq!(
            unsafe { IComponentTrait::getState(&single, ib_stream(&out).as_ptr()) },
            kResultOk
        );
        assert_eq!(out.take_bytes(), encode_state_blob(1.5).to_vec());
        // Controller payload is the same cell in the normalized
        // domain.
        let out = ComWrapper::new(TestStream::new());
        assert_eq!(
            unsafe { IEditControllerTrait::getState(&single, ib_stream(&out).as_ptr()) },
            kResultOk
        );
        assert_eq!(out.take_bytes(), encode_state_blob(0.75).to_vec());
        // Component payload restore (plain 0.5 → normalized 0.25) →
        // `getParamNormalized` reads the same cell.
        let input = ComWrapper::new(TestStream::from_bytes(encode_state_blob(0.5).to_vec()));
        assert_eq!(
            unsafe { IComponentTrait::setState(&single, ib_stream(&input).as_ptr()) },
            kResultOk
        );
        assert_eq!(unsafe { single.getParamNormalized(0) }, 0.25);
        // `setComponentState` (plain domain) folds into the same
        // cell.
        let input = ComWrapper::new(TestStream::from_bytes(encode_state_blob(1.0).to_vec()));
        assert_eq!(
            unsafe { single.setComponentState(ib_stream(&input).as_ptr()) },
            kResultOk
        );
        assert_eq!(unsafe { single.getParamNormalized(0) }, 0.5);
        // Controller payload restore (normalized domain) folds into
        // the same cell.
        let input = ComWrapper::new(TestStream::from_bytes(encode_state_blob(0.75).to_vec()));
        assert_eq!(
            unsafe { IEditControllerTrait::setState(&single, ib_stream(&input).as_ptr()) },
            kResultOk
        );
        assert_eq!(unsafe { single.getParamNormalized(0) }, 0.75);
    }

    /// Malformed blobs are the defined rejection on every state
    /// verb — and leave the cell untouched.
    #[expect(unsafe_code)]
    #[test]
    fn malformed_state_blobs_are_rejected() {
        let single = TestStatefulSingle::new();
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 8]));
        assert_eq!(
            unsafe { IComponentTrait::setState(&single, ib_stream(&bad).as_ptr()) },
            kResultFalse
        );
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 8]));
        assert_eq!(
            unsafe { IEditControllerTrait::setState(&single, ib_stream(&bad).as_ptr()) },
            kResultFalse
        );
        let bad = ComWrapper::new(TestStream::from_bytes(vec![0_u8; 8]));
        assert_eq!(
            unsafe { single.setComponentState(ib_stream(&bad).as_ptr()) },
            kResultFalse
        );
        assert_eq!(unsafe { single.getParamNormalized(0) }, 0.5);
    }

    /// The once-contract: first `setComponentHandler` succeeds, a
    /// second — the double-install bug class — is the defined
    /// rejection.
    #[expect(unsafe_code)]
    #[test]
    fn component_handler_installs_once() {
        let single = TestStatefulSingle::new();
        assert_eq!(
            unsafe { single.setComponentHandler(ptr::null_mut()) },
            kResultOk
        );
        assert_eq!(single.handler_calls.load(Ordering::Acquire), 1);
        assert_eq!(
            unsafe { single.setComponentHandler(ptr::null_mut()) },
            kResultFalse
        );
        assert_eq!(single.handler_calls.load(Ordering::Acquire), 2);
    }

    /// The domain surface is the shared gain law — same parameter
    /// as `test_stateful`.
    #[expect(unsafe_code)]
    #[test]
    fn domain_verbs_match_gain_law() {
        let single = TestStatefulSingle::new();
        assert_eq!(unsafe { single.getParameterCount() }, 1);
        assert_eq!(unsafe { single.normalizedParamToPlain(0, 0.75) }, 1.5);
        assert_eq!(unsafe { single.plainParamToNormalized(0, 1.5) }, 0.75);
        assert_eq!(
            unsafe { single.normalizedParamToPlain(ParamId(7).0, 0.75) },
            0.0
        );
        assert_eq!(
            unsafe { single.setParamNormalized(0, f64::NAN) },
            kInvalidArgument
        );
        assert_eq!(unsafe { single.createView(ptr::null()) }, ptr::null_mut());
    }
}
