//! The VST3 edit controller class: a minimal `IEditController`
//! exposing the single normalized `Gain` parameter.
//!
//! VST3 uses a distributed component/controller architecture — the
//! processor reports this class's CID from `getControllerClassId` and
//! the host instantiates it separately. For the `PoC` the controller is
//! state-light: it holds only the parameter's normalized value
//! (`Cell<f64>` — the trait methods take `&self`).

use std::cell::Cell;
use std::ffi::c_char;
use std::ptr;

use vst3::Steinberg::Vst::{
    IComponentHandler, IEditController, IEditControllerTrait, ParameterInfo,
    ParameterInfo_::ParameterFlags_, String128, TChar,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPlugView, IPluginBaseTrait, TUID, kInvalidArgument, kNotImplemented,
    kResultOk, tresult,
};
use vst3::{Class, uid};

use crate::gain::GAIN_PARAM;
use crate::strings::copy_wstring;

/// The VST3 edit controller for the `PoC` gain plugin.
pub(crate) struct GainController {
    gain: Cell<f64>,
}

impl Class for GainController {
    type Interfaces = (IEditController,);
}

impl GainController {
    /// The controller class ID (`PoC` value — regenerate per real plugin).
    pub(crate) const CID: TUID = uid(0x9D8C_7B6A, 0x5F4E_3D2C, 0x1B0A_9F8E, 0x7D6C_5B4A);

    pub(crate) fn new() -> Self {
        Self {
            gain: Cell::new(GAIN_PARAM.default_value),
        }
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

impl IPluginBaseTrait for GainController {
    #[expect(unsafe_code)]
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IEditControllerTrait for GainController {
    /// Component state streams are not implemented in the `PoC` (the
    /// processor's `setState`/`getState` are also stubs — recorded as
    /// a finding).
    #[expect(unsafe_code)]
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        kNotImplemented
    }

    #[expect(unsafe_code)]
    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
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
        info.id = GAIN_PARAM.id.0;
        copy_wstring(GAIN_PARAM.name, &mut info.title);
        copy_wstring(GAIN_PARAM.name, &mut info.shortTitle);
        copy_wstring(GAIN_PARAM.unit, &mut info.units);
        info.stepCount = 0;
        info.defaultNormalizedValue = GAIN_PARAM.default_value;
        info.unitId = 0;
        info.flags = ParameterFlags_::kCanAutomate;
        kResultOk
    }

    /// Formats the normalized value as a plain decimal string.
    /// (Non-RT UI path — the `to_string` allocation is confined here
    /// and to `getParamValueByString`, recorded in the findings.)
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
        if id != GAIN_PARAM.id.0 {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null — a
        // 128-element `TChar` array.
        let string = unsafe { &mut *string };
        let display = value_normalized.to_string();
        copy_wstring(&display, string);
        kResultOk
    }

    /// Parses a decimal string back to the normalized value.
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
        if id != GAIN_PARAM.id.0 {
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
        let Ok(value) = text.parse::<f64>() else {
            return kInvalidArgument;
        };
        // SAFETY: out-param checked non-null.
        unsafe { *value_normalized = value };
        kResultOk
    }

    /// Normalized and plain domains coincide for the `PoC` gain
    /// (`[0.0, 1.0]` linear) — the mapping is the identity.
    #[expect(unsafe_code)]
    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        if id == GAIN_PARAM.id.0 {
            value_normalized
        } else {
            0.0
        }
    }

    #[expect(unsafe_code)]
    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        if id == GAIN_PARAM.id.0 {
            plain_value
        } else {
            0.0
        }
    }

    #[expect(unsafe_code)]
    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == GAIN_PARAM.id.0 {
            self.gain.get()
        } else {
            0.0
        }
    }

    #[expect(unsafe_code)]
    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != GAIN_PARAM.id.0 {
            return kInvalidArgument;
        }
        self.gain.set(value);
        kResultOk
    }

    /// The `PoC` does not retain the component handler — recorded.
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
