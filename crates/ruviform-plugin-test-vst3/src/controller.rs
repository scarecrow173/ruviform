//! The VST3 edit controller shell shared by every test-suite plugin
//! class: a minimal `IEditController` exposing the class's single
//! declared parameter.
//!
//! VST3 uses a distributed component/controller architecture — the
//! processor reports this class's paired controller CID from
//! `getControllerClassId` and the host instantiates it separately.
//! For these fixtures the controller is state-light: it holds only
//! the parameter's normalized value (`AtomicU64` of `f64` bits — the
//! trait methods take `&self` and may be reached from a different
//! host thread than the processor). The parameter surface is
//! data-driven: [`ParamController::new`] binds one
//! [`ParamInfo`] plus its VST3 `stepCount`, so `test_gain` and
//! `test_fault` share this implementation.
//!
//! ## Domain mapping
//!
//! VST3 parameter APIs speak two domains: *normalized* `[0.0, 1.0]`
//! (`getParamNormalized`/`setParamNormalized`, `getParameterInfo`'s
//! `defaultNormalizedValue`, the `value_normalized` argument of
//! `getParamStringByValue`) and *plain* (the DSP range —
//! `normalizedParamToPlain`/`plainParamToNormalized`, and the string
//! parse/format pair). The mapping is the shared [`ParamInfo`]
//! linear law — deliberately non-identity, so a host that confuses
//! the domains produces measurably wrong results.
//! `getParamStringByValue` formats the **plain** value;
//! `getParamValueByString` parses a **plain** string and returns the
//! normalized result (finite out-of-range plain input clamps per the
//! `ParamInfo` law; non-finite input is rejected).
//!
//! State (de)serialization is data-driven: classes built with
//! [`ParamController::new`] (`test_gain`/`test_fault`) report
//! `kNotImplemented` — the honest no-preset-surface answer — while
//! [`ParamController::new_stateful`] (`test_stateful`) runs the real
//! verbs over the shared [`crate::stateful`] blob codec.

use std::ffi::c_char;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use vst3::Steinberg::Vst::{
    IComponentHandler, IComponentHandler2, IComponentHandler2Trait, IComponentHandlerTrait,
    IEditController, IEditControllerTrait, ParameterInfo, ParameterInfo_::ParameterFlags_,
    String128, TChar,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPlugView, IPluginBaseTrait, TUID, kInvalidArgument, kNotImplemented,
    kResultFalse, kResultOk, tresult,
};
use vst3::{Class, ComRef, ComWrapper, uid};

use crate::editor_view::TestEditorView;
use crate::stateful::{decode_state_blob, encode_state_blob, stream_read_all, stream_write_all};
use crate::strings::{copy_wstring, cstring_eq};
use ruviform_plugin_core::{ParamInfo as NeutralParamInfo, ParamValue};

/// `test_gain`'s controller class ID — a fresh `uid` for the test
/// suite, distinct from the adapter fixture's `GainController::CID`.
pub(crate) const GAIN_CONTROLLER_CID: TUID =
    uid(0x2E8B_5D94, 0xA1F6_7C38, 0x4D09_8B2E, 0xC6F4_3A17);

/// `test_editor`'s controller class ID — a fresh `uid` for the test
/// suite, distinct from `test_gain`/`test_fault` CIDs.
pub(crate) const EDITOR_CONTROLLER_CID: TUID =
    uid(0x5E7A_9C31, 0x2D8F_4B67, 0xE1A3_59D4, 0x8C6B_2F07);

/// `test_instrument`'s controller class ID — a fresh `uid` for the
/// test suite, distinct from every other fixture CID.
pub(crate) const INSTRUMENT_CONTROLLER_CID: TUID =
    uid(0x9C4D_2F81, 0x7E3A_5B96, 0xD8F1_4A62, 0x3B7E_9C05);

/// A VST3 edit controller bound to one declared parameter.
///
/// `step_count` is the VST3 `ParameterInfo::stepCount` — `0` for a
/// continuous parameter, `N` for a parameter with `N + 1` discrete
/// steps (the host maps any positive value to the `STEPPED` flag).
pub(crate) struct ParamController {
    /// The declared parameter this controller exposes.
    param: &'static NeutralParamInfo,
    /// `ParameterInfo::stepCount` reported to the host.
    step_count: i32,
    /// The parameter's normalized value (`f64` bits).
    value: AtomicU64,
    /// Whether the state verbs are live — `true` only for
    /// `test_stateful`; `test_gain`/`test_fault` keep the honest
    /// `kNotImplemented`.
    stateful: bool,
    /// The host's `IComponentHandler` — a *borrowed* pointer the VST3
    /// contract keeps valid for the object's lifetime (the host owns
    /// the reference), retained so the state verbs can announce a
    /// dirty document (`IComponentHandler2::setDirty`) and the
    /// restored parameter value (`performEdit`). Null until the host
    /// installs it; stateless classes never announce.
    handler: AtomicPtr<IComponentHandler>,
}

impl Class for ParamController {
    type Interfaces = (IEditController,);
}

impl ParamController {
    /// A controller for `param` reporting `step_count` — stateless:
    /// the state verbs stay `kNotImplemented`.
    pub(crate) fn new(param: &'static NeutralParamInfo, step_count: i32) -> Self {
        Self {
            param,
            step_count,
            value: AtomicU64::new(
                param
                    .default_normalized()
                    .map_or(0.5, ParamValue::get)
                    .to_bits(),
            ),
            stateful: false,
            handler: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// A stateful controller — the same parameter surface as
    /// [`ParamController::new`] but with live `setState`/`getState`/
    /// `setComponentState` verbs over the [`crate::stateful`] blob
    /// codec (`test_stateful` only).
    pub(crate) fn new_stateful(param: &'static NeutralParamInfo, step_count: i32) -> Self {
        let mut controller = Self::new(param, step_count);
        controller.stateful = true;
        controller
    }
}

/// Reads a NUL-terminated `TChar` string's length, bounded by
/// `MAX_LEN` so a missing terminator cannot read past the field.
///
/// # Safety
///
/// `string` must point to at least `MAX_LEN` readable `TChar`s (the
/// VST3 `String128` contract provides 128).
#[expect(unsafe_code)]
unsafe fn len_wstring<const MAX_LEN: usize>(string: *const TChar) -> usize {
    let mut len = 0;
    // SAFETY: bounded walk over `MAX_LEN` elements of a valid buffer.
    while len < MAX_LEN && unsafe { *string.add(len) } != 0 {
        len += 1;
    }
    len
}

impl IPluginBaseTrait for ParamController {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IEditControllerTrait for ParamController {
    /// `test_stateful`'s real state verbs over the shared
    /// [`crate::stateful`] codec; every other class reports the
    /// honest `kNotImplemented`.
    ///
    /// `setComponentState` receives the *component* payload (plain
    /// gain domain) — the adapter re-serializes it through
    /// `component.getState` before the hand-off — and folds it into
    /// this controller's normalized mirror via the `ParamInfo` law.
    /// `setState`/`getState` carry the *controller* payload
    /// (normalized domain). Malformed blobs are the defined
    /// `kResultFalse` rejection.
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        if !self.stateful {
            return kNotImplemented;
        }
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        let bytes = match unsafe { stream_read_all(state) } {
            Ok(bytes) => bytes,
            Err(result) => return result,
        };
        let Some(plain) = decode_state_blob(&bytes) else {
            return kResultFalse;
        };
        // `normalize` clamps finite out-of-range plain values into
        // the normalized domain; non-finite cannot reach here (the
        // codec rejects it) — the `else` is the defined backstop.
        let Ok(normalized) = self.param.normalize(plain) else {
            return kResultFalse;
        };
        self.adopt_and_announce(normalized);
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        if !self.stateful {
            return kNotImplemented;
        }
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
        self.store_and_mark(value);
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        if !self.stateful {
            return kNotImplemented;
        }
        let normalized = f64::from_bits(self.value.load(Ordering::Acquire));
        let bytes = encode_state_blob(normalized);
        // SAFETY: `state` is a valid `IBStream` for the call's
        // duration per the VST3 state-verb contract.
        unsafe { stream_write_all(state, &bytes) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        1
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterInfo(&self, param_index: i32, info: *mut ParameterInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        if param_index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.id = self.param.id.0;
        copy_wstring(self.param.name, &mut info.title);
        copy_wstring(self.param.name, &mut info.shortTitle);
        copy_wstring(self.param.unit, &mut info.units);
        info.stepCount = self.step_count;
        info.defaultNormalizedValue = self.param.default_normalized().map_or(0.5, ParamValue::get);
        info.unitId = 0;
        info.flags = ParameterFlags_::kCanAutomate;
        kResultOk
    }

    /// Formats the **plain**-domain value of the normalized input —
    /// VST3 displays plain values to the user while automation speaks
    /// normalized. (Non-RT UI path — the `to_string` allocation is
    /// confined here and to `getParamValueByString`.)
    #[expect(unsafe_code)]
    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        value_normalized: f64,
        string: *mut String128,
    ) -> tresult {
        if string.is_null() {
            return kInvalidArgument;
        }
        if id != self.param.id.0 {
            return kInvalidArgument;
        }
        // Non-finite normalized input is a defined rejection — the
        // `ParamValue` NaN policy.
        let Ok(normalized) = ParamValue::new(value_normalized) else {
            return kInvalidArgument;
        };
        // SAFETY: caller-provided out-param, checked non-null — a
        // 128-element `TChar` array.
        let string = unsafe { &mut *string };
        let display = self.param.denormalize(normalized).to_string();
        copy_wstring(&display, string);
        kResultOk
    }

    /// Parses a **plain**-domain decimal string and returns the
    /// normalized result — the inverse of `getParamStringByValue`.
    /// Finite out-of-range plain input clamps per the `ParamInfo`
    /// law; non-finite input and unparseable text are rejected.
    #[expect(unsafe_code)]
    unsafe fn getParamValueByString(
        &self,
        id: u32,
        string: *mut TChar,
        value_normalized: *mut f64,
    ) -> tresult {
        if string.is_null() || value_normalized.is_null() {
            return kInvalidArgument;
        }
        if id != self.param.id.0 {
            return kInvalidArgument;
        }
        // SAFETY: `string` is a valid 128-element `TChar` buffer per
        // the `String128` contract; `len_wstring` is bounded.
        let len = unsafe { len_wstring::<128>(string) };
        // SAFETY: `len` elements of the same live buffer.
        let units = unsafe { std::slice::from_raw_parts(string, len) };
        let Ok(text) = String::from_utf16(units) else {
            return kInvalidArgument;
        };
        let Ok(plain) = text.parse::<f64>() else {
            return kInvalidArgument;
        };
        // `normalize` rejects non-finite plain values and clamps
        // finite out-of-range ones into the normalized domain.
        let Ok(normalized) = self.param.normalize(plain) else {
            return kInvalidArgument;
        };
        // SAFETY: out-param checked non-null.
        unsafe { *value_normalized = normalized.get() };
        kResultOk
    }

    /// Normalized → plain via the `ParamInfo` linear law. Non-finite
    /// input maps to `0.0` — the defined fallback for a value outside
    /// the `ParamValue` domain.
    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        if id != self.param.id.0 {
            return 0.0;
        }
        ParamValue::new(value_normalized).map_or(0.0, |v| self.param.denormalize(v))
    }

    /// Plain → normalized via the `ParamInfo` linear law (clamped).
    /// Non-finite input maps to `0.0`.
    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        if id != self.param.id.0 {
            return 0.0;
        }
        self.param
            .normalize(plain_value)
            .map_or(0.0, ParamValue::get)
    }

    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == self.param.id.0 {
            f64::from_bits(self.value.load(Ordering::Acquire))
        } else {
            0.0
        }
    }

    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != self.param.id.0 {
            return kInvalidArgument;
        }
        // The normalized domain contract: a non-finite or out-of-range
        // value is a defined rejection, not a stored NaN.
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return kInvalidArgument;
        }
        self.value.store(value.to_bits(), Ordering::Release);
        kResultOk
    }

    /// Retains the borrowed component-handler pointer — the stateful
    /// verbs announce their restores through it (`performEdit` for
    /// the changed value, `IComponentHandler2::setDirty` for the
    /// dirty edge; Issue #891). Stateless classes retain it too —
    /// they just never fire.
    #[expect(unsafe_code)]
    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        self.handler.store(handler, Ordering::Release);
        kResultOk
    }

    /// No GUI — `createView` returns null per the VST3 contract.
    #[expect(unsafe_code)]
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        ptr::null_mut()
    }
}

impl ParamController {
    /// The stored component handler as a borrowed `ComRef` — `None`
    /// when the host never installed one.
    #[expect(unsafe_code)]
    fn host_handler(&self) -> Option<ComRef<'_, IComponentHandler>> {
        // SAFETY: a non-null stored handler is a live host-provided
        // `IComponentHandler` per the setComponentHandler contract;
        // `ComRef::from_raw` borrows without touching the refcount.
        unsafe { ComRef::from_raw(self.handler.load(Ordering::Acquire)) }
    }

    /// `setComponentState`'s tail: adopt the normalized value and
    /// announce — `performEdit` echoes it to the host and
    /// `IComponentHandler2::setDirty` marks the document dirty. The
    /// call itself is the change signal (the documented restore
    /// sequence runs `component.setState` → `setComponentState` only
    /// when component state was (re)applied), so the announce is
    /// unconditional: the process-path param flow
    /// (`IParameterChanges`) never reaches this controller's adopted
    /// mirror, so a stored-value comparison would wrongly suppress
    /// restores that genuinely changed the component. Hosts that
    /// never installed a handler, or whose handler has no
    /// `IComponentHandler2`, simply never see the request — the
    /// restore still stores.
    #[expect(unsafe_code)]
    fn adopt_and_announce(&self, normalized: ParamValue) {
        self.value
            .store(normalized.get().to_bits(), Ordering::Release);
        let Some(handler) = self.host_handler() else {
            return;
        };
        let _ = unsafe { handler.performEdit(self.param.id.0, normalized.get()) };
        if let Some(handler2) = handler.cast::<IComponentHandler2>() {
            let _ = unsafe { handler2.setDirty(1) };
        }
    }

    /// `setState`'s tail: store the controller-private state and
    /// `setDirty` when it actually changed — never a `performEdit`
    /// echo (controller state is not a parameter edit the host must
    /// reflect).
    #[expect(unsafe_code)]
    fn store_and_mark(&self, normalized: ParamValue) {
        let previous = self
            .value
            .swap(normalized.get().to_bits(), Ordering::AcqRel);
        if previous == normalized.get().to_bits() {
            return;
        }
        let Some(handler) = self.host_handler() else {
            return;
        };
        if let Some(handler2) = handler.cast::<IComponentHandler2>() {
            let _ = unsafe { handler2.setDirty(1) };
        }
    }
}

/// `test_editor`'s edit controller — the suite's editor fixture
/// (Issue #630 AC-4 plugin side) carrying the shared `Fault`
/// parameter surface (Issue #885). Every parameter/state verb
/// delegates to the same [`ParamController`] law `test_fault` binds
/// (`getParameterCount` → `1`, the stepped `Fault` descriptor, the
/// `ParamInfo` domain mapping, honest `kNotImplemented` state
/// streams); `createView` stays this class's own: it returns the
/// owned [`IPlugView`] for `"editor"` only — the real child-HWND
/// surface the production host's `open_gui` attaches.
pub(crate) struct TestEditorController {
    /// The declared-parameter surface — a [`ParamController`] bound
    /// to [`crate::fault::FAULT_PARAM`].
    params: ParamController,
}

impl Class for TestEditorController {
    type Interfaces = (IEditController,);
}

impl TestEditorController {
    /// A controller exposing the shared `Fault` parameter — the same
    /// surface `test_fault`'s controller reports.
    pub(crate) fn new() -> Self {
        Self {
            params: ParamController::new(
                &crate::fault::FAULT_PARAM,
                crate::fault::FAULT_STEP_COUNT,
            ),
        }
    }
}

impl IPluginBaseTrait for TestEditorController {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.initialize(context) }
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.terminate() }
    }
}

impl IEditControllerTrait for TestEditorController {
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.setComponentState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.setState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.getState(state) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.getParameterCount() }
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterInfo(&self, param_index: i32, info: *mut ParameterInfo) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.getParameterInfo(param_index, info) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        value_normalized: f64,
        string: *mut String128,
    ) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe {
            self.params
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
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe {
            self.params
                .getParamValueByString(id, string, value_normalized)
        }
    }

    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.normalizedParamToPlain(id, value_normalized) }
    }

    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.plainParamToNormalized(id, plain_value) }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.getParamNormalized(id) }
    }

    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        // SAFETY: forwarded to the delegate under the same contract.
        unsafe { self.params.setParamNormalized(id, value) }
    }

    /// The fixture does not retain the component handler — recorded.
    #[expect(unsafe_code)]
    unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
        kResultOk
    }

    /// `"editor"` returns a fresh owned `IPlugView` (the host takes
    /// ownership via `ComPtr::from_raw`); every other name — including
    /// null — returns null per the VST3 contract.
    #[expect(unsafe_code)]
    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        // SAFETY: `name` is a VST3 view-type `FIDString` or null per
        // contract.
        if !unsafe { cstring_eq(name, "editor") } {
            return ptr::null_mut();
        }
        let view = ComWrapper::new(TestEditorView::new());
        match view.to_com_ptr::<IPlugView>() {
            Some(ptr) => ptr.into_raw(),
            None => ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gain::GAIN_PARAM;
    use vst3::Steinberg::Vst::String128;

    /// A `&str` as a NUL-terminated `String128` — mirrors what a host
    /// supplies to `getParamValueByString`.
    fn wstring(text: &str) -> String128 {
        let mut buf: String128 = [0; 128];
        for (i, unit) in text.encode_utf16().enumerate() {
            buf[i] = unit;
        }
        buf
    }

    /// The normalized↔plain mapping is the non-identity `ParamInfo`
    /// law — the oracle that catches a domain-confused host.
    #[expect(unsafe_code)]
    #[test]
    fn param_domain_mapping_is_non_identity() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        // SAFETY: no pointers involved.
        unsafe {
            assert_eq!(controller.normalizedParamToPlain(0, 0.5), 1.0);
            assert_eq!(controller.normalizedParamToPlain(0, 0.25), 0.5);
            assert_eq!(controller.normalizedParamToPlain(0, 1.0), 2.0);
            assert_eq!(controller.plainParamToNormalized(0, 1.0), 0.5);
            assert_eq!(controller.plainParamToNormalized(0, 2.0), 1.0);
            // Foreign ids map to the defined fallback.
            assert_eq!(controller.normalizedParamToPlain(9, 0.5), 0.0);
            assert_eq!(controller.plainParamToNormalized(9, 1.0), 0.0);
        }
    }

    /// `getParameterInfo` reports the plain range through the
    /// normalized default (`0.5` ↔ plain `1.0`).
    #[expect(unsafe_code)]
    #[test]
    fn parameter_info_reports_gain_descriptor() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        let mut info: ParameterInfo = unsafe { core::mem::zeroed() };
        // SAFETY: `info` is a valid out-param.
        unsafe {
            assert_eq!(controller.getParameterInfo(0, &raw mut info), kResultOk);
            assert_eq!(
                controller.getParameterInfo(1, &raw mut info),
                kInvalidArgument
            );
        }
        assert_eq!(info.id, 0);
        assert_eq!(info.defaultNormalizedValue, 0.5);
        assert_eq!(info.stepCount, 0);
        assert_eq!(info.flags, ParameterFlags_::kCanAutomate);
    }

    /// Non-finite and out-of-range normalized values are defined
    /// rejections, never stored — the same domain contract the event
    /// path enforces via `ParamValue`.
    #[expect(unsafe_code)]
    #[test]
    fn set_param_normalized_enforces_the_domain() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        // SAFETY: no pointers involved.
        unsafe {
            assert_eq!(controller.setParamNormalized(0, 0.75), kResultOk);
            assert_eq!(controller.getParamNormalized(0), 0.75);

            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5, -0.1] {
                assert_eq!(controller.setParamNormalized(0, bad), kInvalidArgument);
            }
            assert_eq!(controller.setParamNormalized(9, 0.5), kInvalidArgument);
            // Rejected values were never stored.
            assert_eq!(controller.getParamNormalized(0), 0.75);
        }
    }

    /// `getParamStringByValue` formats the plain domain:
    /// normalized `0.5` → plain `1.0` → `"1"`.
    #[expect(unsafe_code)]
    #[test]
    fn param_string_formats_plain_domain() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        let mut out: String128 = [0; 128];
        // SAFETY: `out` is a valid 128-`TChar` buffer.
        unsafe {
            assert_eq!(
                controller.getParamStringByValue(0, 0.5, &raw mut out),
                kResultOk
            );
            let len = len_wstring::<128>(out.as_ptr());
            let text = String::from_utf16(&out[..len]).unwrap();
            assert_eq!(text, "1");

            // Non-finite normalized input is rejected.
            assert_eq!(
                controller.getParamStringByValue(0, f64::NAN, &raw mut out),
                kInvalidArgument
            );
            assert_eq!(
                controller.getParamStringByValue(9, 0.5, &raw mut out),
                kInvalidArgument
            );
        }
    }

    /// `getParamValueByString` parses the plain domain and returns
    /// normalized: `"1.5"` → `0.75`, `"2.0"` → `1.0`; `"NaN"`/`"inf"`
    /// and unparseable text are rejected; finite out-of-range plain
    /// input clamps per the `ParamInfo` law.
    #[expect(unsafe_code)]
    #[test]
    fn param_value_by_string_parses_plain_domain() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        let mut out = -1.0_f64;
        // SAFETY: `text` buffers are valid 128-`TChar` arrays; `out`
        // is a valid out-param.
        unsafe {
            let mut text = wstring("1.5");
            assert_eq!(
                controller.getParamValueByString(0, text.as_mut_ptr(), &raw mut out),
                kResultOk
            );
            assert_eq!(out, 0.75);

            let mut text = wstring("2.0");
            assert_eq!(
                controller.getParamValueByString(0, text.as_mut_ptr(), &raw mut out),
                kResultOk
            );
            assert_eq!(out, 1.0);

            // Out-of-range plain clamps into the normalized domain.
            let mut text = wstring("4.0");
            assert_eq!(
                controller.getParamValueByString(0, text.as_mut_ptr(), &raw mut out),
                kResultOk
            );
            assert_eq!(out, 1.0);

            for bad in ["NaN", "inf", "-inf", "abc"] {
                let mut text = wstring(bad);
                assert_eq!(
                    controller.getParamValueByString(0, text.as_mut_ptr(), &raw mut out),
                    kInvalidArgument,
                    "{bad} must be rejected"
                );
            }
            // Never clobbered by a rejected parse.
            assert_eq!(out, 1.0);
        }
    }

    /// State streams are honestly `kNotImplemented`.
    #[expect(unsafe_code)]
    #[test]
    fn state_stream_reports_not_implemented() {
        let controller = ParamController::new(&GAIN_PARAM, 0);
        // SAFETY: null `IBStream` is unused by the stubs.
        unsafe {
            assert_eq!(
                controller.setComponentState(ptr::null_mut()),
                kNotImplemented
            );
            assert_eq!(controller.setState(ptr::null_mut()), kNotImplemented);
            assert_eq!(controller.getState(ptr::null_mut()), kNotImplemented);
        }
    }

    /// `test_editor`'s controller declares the shared `Fault`
    /// parameter — the same surface `test_fault`'s `ParamController`
    /// reports: `getParameterCount == 1`, the stepped descriptor
    /// (id `0`, `stepCount == 7`, `kCanAutomate`, default normalized
    /// `0.0`), the `ParamInfo` domain mapping (`[0.0, 8.0]` plain —
    /// normalized `0.5` ↔ plain `4.0`), and `setParamNormalized`
    /// enforcing the normalized domain.
    #[expect(unsafe_code)]
    #[test]
    fn editor_controller_reports_fault_param() {
        let controller = TestEditorController::new();
        let mut info: ParameterInfo = unsafe { core::mem::zeroed() };
        // SAFETY: `info` is a valid out-param; the plain calls take
        // no pointers.
        unsafe {
            assert_eq!(controller.getParameterCount(), 1);
            assert_eq!(controller.getParameterInfo(0, &raw mut info), kResultOk);
            assert_eq!(
                controller.getParameterInfo(1, &raw mut info),
                kInvalidArgument
            );
            assert_eq!(info.id, 0);
            assert_eq!(info.defaultNormalizedValue, 0.0);
            assert_eq!(info.stepCount, 7);
            assert_eq!(info.flags, ParameterFlags_::kCanAutomate);

            // The `Fault` plain domain is `[0.0, 8.0]` — the same
            // non-identity `ParamInfo` law `test_fault` binds.
            assert_eq!(controller.normalizedParamToPlain(0, 0.5), 4.0);
            assert_eq!(controller.normalizedParamToPlain(0, 0.0), 0.0);
            assert_eq!(controller.normalizedParamToPlain(0, 1.0), 8.0);
            assert_eq!(controller.plainParamToNormalized(0, 4.0), 0.5);
            assert_eq!(controller.plainParamToNormalized(0, 8.0), 1.0);
            // Foreign ids map to the defined fallback.
            assert_eq!(controller.normalizedParamToPlain(9, 0.5), 0.0);
            assert_eq!(controller.plainParamToNormalized(9, 1.0), 0.0);

            // The normalized domain contract: accepted values are
            // stored; non-finite/out-of-range are defined rejections.
            assert_eq!(controller.setParamNormalized(0, 0.5), kResultOk);
            assert_eq!(controller.getParamNormalized(0), 0.5);
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5, -0.1] {
                assert_eq!(controller.setParamNormalized(0, bad), kInvalidArgument);
            }
            assert_eq!(controller.setParamNormalized(9, 0.5), kInvalidArgument);
            assert_eq!(controller.getParamNormalized(0), 0.5);
        }
    }

    /// `createView("editor")` returns a non-null **owned** `IPlugView`
    /// — released here through `ComPtr::from_raw`, the same ownership
    /// transfer the production host performs. Every other name —
    /// including null — returns null.
    #[expect(unsafe_code)]
    #[test]
    fn editor_controller_create_view_dispatches_on_name() {
        use std::ffi::CString;

        use vst3::ComPtr;
        use vst3::Steinberg::Vst::ViewType;

        let controller = TestEditorController::new();
        let editor = CString::new("editor").unwrap();
        let other = CString::new("other").unwrap();
        // SAFETY: `editor`/`other` are live NUL-terminated strings;
        // returned pointers are owned by the test and released via
        // `ComPtr::from_raw` — never leaked, never double-freed.
        unsafe {
            let view = controller.createView(ViewType::kEditor);
            assert!(!view.is_null(), "createView(\"editor\") must return a view");
            drop(ComPtr::from_raw(view));

            // The same dispatch through the literal bytes — proves
            // the comparison is content-based, not pointer-based.
            let view = controller.createView(editor.as_ptr());
            assert!(!view.is_null());
            drop(ComPtr::from_raw(view));

            assert!(controller.createView(other.as_ptr()).is_null());
            assert!(controller.createView(ptr::null()).is_null());
        }
    }
}
