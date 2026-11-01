//! The VST3 plugin factory: `IPluginFactory` + `IPluginFactory2` +
//! `IPluginFactory3` describing and instantiating the two classes the
//! module ships (component + controller).

use std::ffi::c_void;

use vst3::Steinberg::{
    FIDString, FUnknown, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3,
    IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo_, PClassInfo2, PClassInfoW,
    PFactoryInfo, PFactoryInfo_, TUID, kInvalidArgument, kResultOk, tresult,
};
use vst3::{Class, ComWrapper};

use crate::component::GainComponent;
use crate::controller::GainController;
use crate::sdk_enum_i32;
use crate::strings::{copy_cstring, copy_wstring};

/// The VST3 plugin factory for the `PoC` gain module.
pub(crate) struct GainFactory;

impl Class for GainFactory {
    type Interfaces = (IPluginFactory, IPluginFactory2, IPluginFactory3);
}

impl GainFactory {
    /// A fresh factory — stateless.
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl IPluginFactoryTrait for GainFactory {
    #[expect(unsafe_code)]
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        copy_cstring("Ruviform", &mut info.vendor);
        copy_cstring("https://github.com/scarecrow173/ruviform", &mut info.url);
        copy_cstring("ruviform@example.invalid", &mut info.email);
        info.flags = sdk_enum_i32(PFactoryInfo_::FactoryFlags_::kUnicode);
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn countClasses(&self) -> i32 {
        2
    }

    /// Class 0 = audio component, class 1 = edit controller.
    #[expect(unsafe_code)]
    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        match index {
            0 => {
                info.cid = GainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("Gain", &mut info.name);
            }
            1 => {
                info.cid = GainController::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("Gain Controller", &mut info.name);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }

    /// Instantiates a class by CID and queries the requested interface
    /// — the only place a raw `ComWrapper` pointer escapes the crate.
    #[expect(unsafe_code)]
    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        if cid.is_null() || iid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: per the `createInstance` contract `cid`/`iid` point to
        // `TUID`s (`FIDString` aliases `[char8; 16]`); `obj` is a valid
        // out-pointer.
        let instance = match unsafe { *cid.cast::<TUID>() } {
            GainComponent::CID => ComWrapper::new(GainComponent::new()).to_com_ptr::<FUnknown>(),
            GainController::CID => ComWrapper::new(GainController::new()).to_com_ptr::<FUnknown>(),
            _ => None,
        };
        let Some(instance) = instance else {
            return kInvalidArgument;
        };
        // SAFETY: `instance` is a live COM object whose vtbl owns a
        // valid `queryInterface`; `iid`/`obj` are valid per contract.
        // The returned interface pointer's refcount is transferred to
        // the caller; `instance` (the temporary `ComPtr`) is dropped
        // after the query, which is correct COM bookkeeping.
        let ptr = instance.as_ptr();
        unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid.cast::<TUID>(), obj) }
    }
}

impl IPluginFactory2Trait for GainFactory {
    /// `PClassInfo2` — the VST3.5+ info variant; all fields are `char8`.
    #[expect(unsafe_code)]
    unsafe fn getClassInfo2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        match index {
            0 => {
                info.cid = GainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("Gain", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_cstring("Ruviform", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            1 => {
                info.cid = GainController::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("Gain Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_cstring("Ruviform", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }
}

impl IPluginFactory3Trait for GainFactory {
    /// `PClassInfoW` — the wide variant: `category`/`subCategories`
    /// stay `char8`, the remaining strings are `TChar`.
    #[expect(unsafe_code)]
    unsafe fn getClassInfoUnicode(&self, index: i32, info: *mut PClassInfoW) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        match index {
            0 => {
                info.cid = GainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_wstring("Gain", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_wstring("Ruviform", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            1 => {
                info.cid = GainController::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_wstring("Gain Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_wstring("Ruviform", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }

    /// No host-context features needed by the `PoC` — the call is
    /// acknowledged per the `IPluginFactory3` contract.
    #[expect(unsafe_code)]
    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
