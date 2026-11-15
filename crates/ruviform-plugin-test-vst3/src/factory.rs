//! The VST3 plugin factory: `IPluginFactory` + `IPluginFactory2` +
//! `IPluginFactory3` describing and instantiating the six classes
//! the module ships (`test_gain`/`test_fault`/`test_editor`
//! component + controller pairs).

use std::ffi::c_void;

use vst3::Steinberg::{
    FIDString, FUnknown, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3,
    IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo_, PClassInfo2, PClassInfoW,
    PFactoryInfo, PFactoryInfo_, TUID, kInvalidArgument, kResultOk, tresult,
};
use vst3::{Class, ComWrapper};

use crate::component::TestGainComponent;
use crate::controller::{
    EDITOR_CONTROLLER_CID, GAIN_CONTROLLER_CID, ParamController, TestEditorController,
};
use crate::editor::TestEditorComponent;
use crate::fault::{FAULT_CONTROLLER_CID, FAULT_PARAM, FAULT_STEP_COUNT, TestFaultComponent};
use crate::gain::GAIN_PARAM;
use crate::sdk_enum_i32;
use crate::strings::{copy_cstring, copy_wstring};

/// The VST3 plugin factory for the test-suite module.
pub(crate) struct TestGainFactory;

impl Class for TestGainFactory {
    type Interfaces = (IPluginFactory, IPluginFactory2, IPluginFactory3);
}

impl TestGainFactory {
    /// A fresh factory — stateless.
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl IPluginFactoryTrait for TestGainFactory {
    #[expect(unsafe_code)]
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        copy_cstring("Ruviform Test", &mut info.vendor);
        copy_cstring("https://github.com/scarecrow173/ruviform", &mut info.url);
        copy_cstring("ruviform-test@example.invalid", &mut info.email);
        info.flags = sdk_enum_i32(PFactoryInfo_::FactoryFlags_::kUnicode);
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn countClasses(&self) -> i32 {
        6
    }

    /// Classes 0/1 = `test_gain`, 2/3 = `test_fault`, 4/5 =
    /// `test_editor` — component + controller pairs in suite order.
    #[expect(unsafe_code)]
    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        match index {
            0 => {
                info.cid = TestGainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_gain", &mut info.name);
            }
            1 => {
                info.cid = GAIN_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_gain Controller", &mut info.name);
            }
            2 => {
                info.cid = TestFaultComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_fault", &mut info.name);
            }
            3 => {
                info.cid = FAULT_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_fault Controller", &mut info.name);
            }
            4 => {
                info.cid = TestEditorComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_editor", &mut info.name);
            }
            5 => {
                info.cid = EDITOR_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_editor Controller", &mut info.name);
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
            TestGainComponent::CID => {
                ComWrapper::new(TestGainComponent::new()).to_com_ptr::<FUnknown>()
            }
            GAIN_CONTROLLER_CID => {
                ComWrapper::new(ParamController::new(&GAIN_PARAM, 0)).to_com_ptr::<FUnknown>()
            }
            TestFaultComponent::CID => {
                ComWrapper::new(TestFaultComponent::new()).to_com_ptr::<FUnknown>()
            }
            FAULT_CONTROLLER_CID => {
                ComWrapper::new(ParamController::new(&FAULT_PARAM, FAULT_STEP_COUNT))
                    .to_com_ptr::<FUnknown>()
            }
            TestEditorComponent::CID => {
                ComWrapper::new(TestEditorComponent::new()).to_com_ptr::<FUnknown>()
            }
            EDITOR_CONTROLLER_CID => ComWrapper::new(TestEditorController).to_com_ptr::<FUnknown>(),
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

impl IPluginFactory2Trait for TestGainFactory {
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
                info.cid = TestGainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_gain", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            1 => {
                info.cid = GAIN_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_gain Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            2 => {
                info.cid = TestFaultComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_fault", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            3 => {
                info.cid = FAULT_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_fault Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            4 => {
                info.cid = TestEditorComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_cstring("test_editor", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            5 => {
                info.cid = EDITOR_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_cstring("test_editor Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_cstring("Ruviform Test", &mut info.vendor);
                copy_cstring("0.0.0", &mut info.version);
                copy_cstring("VST 3.x", &mut info.sdkVersion);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }
}

impl IPluginFactory3Trait for TestGainFactory {
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
                info.cid = TestGainComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_wstring("test_gain", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            1 => {
                info.cid = GAIN_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_wstring("test_gain Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            2 => {
                info.cid = TestFaultComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_wstring("test_fault", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            3 => {
                info.cid = FAULT_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_wstring("test_fault Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            4 => {
                info.cid = TestEditorComponent::CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Audio Module Class", &mut info.category);
                copy_wstring("test_editor", &mut info.name);
                info.classFlags = 0;
                copy_cstring("Fx", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            5 => {
                info.cid = EDITOR_CONTROLLER_CID;
                info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
                copy_cstring("Component Controller Class", &mut info.category);
                copy_wstring("test_editor Controller", &mut info.name);
                info.classFlags = 0;
                copy_cstring("", &mut info.subCategories);
                copy_wstring("Ruviform Test", &mut info.vendor);
                copy_wstring("0.0.0", &mut info.version);
                copy_wstring("VST 3.x", &mut info.sdkVersion);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }

    /// No host-context features needed by the `test_gain` fixture —
    /// the call is acknowledged per the `IPluginFactory3` contract.
    #[expect(unsafe_code)]
    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
