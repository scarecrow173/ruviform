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
use std::sync::atomic::{AtomicU64, Ordering};

use vst3::Steinberg::Vst::{
    IComponentHandler, IEditController, IEditControllerTrait, ParameterInfo,
    ParameterInfo_::ParameterFlags_, String128, TChar,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPlugView, IPluginBaseTrait, TUID, kInvalidArgument, kNotImplemented,
    kResultFalse, kResultOk, tresult,
};
use vst3::{Class, ComWrapper, uid};

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
        self.value
            .store(normalized.get().to_bits(), Ordering::Release);
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
        self.value.store(value.get().to_bits(), Ordering::Release);
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

    /// The fixture does not retain the component handler — recorded.
    #[expect(unsafe_code)]
    unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
        kResultOk
    }

    /// No GUI — `createView` returns null per the VST3 contract.
    #[expect(unsafe_code)]
    unsafe fn createView(&self, _name: *const c_char) -> *mut IPlugView {
        ptr::null_mut()
    }
}

/// `test_editor`'s edit controller — the suite's editor fixture
/// (Issue #630 AC-4 plugin side). Unlike [`ParamController`] it
/// declares **no parameters** (`getParameterCount` → `0`, every
/// parameter verb an honest rejection) and its `createView` returns
/// the owned [`IPlugView`] for `"editor"` only — the real
/// child-HWND surface the production host's `open_gui` attaches.
///
/// State (de)serialization reports `kNotImplemented`, matching the
/// other fixtures' no-preset-surface policy.
pub(crate) struct TestEditorController;

impl Class for TestEditorController {
    type Interfaces = (IEditController,);
}

impl IPluginBaseTrait for TestEditorController {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IEditControllerTrait for TestEditorController {
    /// No preset surface — reported honestly as `kNotImplemented`.
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }

    /// `test_editor` declares no parameters.
    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        0
    }

    /// No parameters exist — every index is out of range.
    #[expect(unsafe_code)]
    unsafe fn getParameterInfo(&self, _param_index: i32, _info: *mut ParameterInfo) -> tresult {
        kInvalidArgument
    }

    /// No parameters exist — no value can be formatted.
    #[expect(unsafe_code)]
    unsafe fn getParamStringByValue(
        &self,
        _id: u32,
        _value_normalized: f64,
        _string: *mut String128,
    ) -> tresult {
        kInvalidArgument
    }

    /// No parameters exist — no string can be parsed.
    #[expect(unsafe_code)]
    unsafe fn getParamValueByString(
        &self,
        _id: u32,
        _string: *mut TChar,
        _value_normalized: *mut f64,
    ) -> tresult {
        kInvalidArgument
    }

    /// No parameters exist — the defined fallback `0.0`.
    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, _id: u32, _value_normalized: f64) -> f64 {
        0.0
    }

    /// No parameters exist — the defined fallback `0.0`.
    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, _id: u32, _plain_value: f64) -> f64 {
        0.0
    }

    /// No parameters exist — the defined fallback `0.0`.
    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, _id: u32) -> f64 {
        0.0
    }

    /// No parameters exist — every set is a defined rejection.
    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, _id: u32, _value: f64) -> tresult {
        kInvalidArgument
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

    /// `test_editor`'s controller declares zero parameters — the
    /// surface the component's sentinel `param_id` mirrors.
    #[expect(unsafe_code)]
    #[test]
    fn editor_controller_reports_zero_params() {
        let controller = TestEditorController;
        // SAFETY: no pointers involved / `info` unused on rejection.
        unsafe {
            assert_eq!(controller.getParameterCount(), 0);
            assert_eq!(controller.getParamNormalized(0), 0.0);
            assert_eq!(controller.setParamNormalized(0, 0.5), kInvalidArgument);
            assert_eq!(controller.normalizedParamToPlain(0, 0.5), 0.0);
            assert_eq!(controller.plainParamToNormalized(0, 1.0), 0.0);
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

        let controller = TestEditorController;
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
