//! The VST3 plugin factory: `IPluginFactory` + `IPluginFactory2` +
//! `IPluginFactory3` describing and instantiating the eleven classes
//! the module ships (`test_gain`/`test_fault`/`test_editor`/
//! `test_stateful`/`test_instrument` component + controller pairs, plus
//! the listed-but-uncreatable `test_spawn_fail` class).

use std::ffi::c_void;

use vst3::Steinberg::{
    FIDString, FUnknown, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3,
    IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo_, PClassInfo2, PClassInfoW,
    PFactoryInfo, PFactoryInfo_, TUID, kInvalidArgument, kResultOk, tresult,
};
use vst3::{Class, ComWrapper};

use crate::component::TestGainComponent;
use crate::controller::{
    EDITOR_CONTROLLER_CID, GAIN_CONTROLLER_CID, INSTRUMENT_CONTROLLER_CID, ParamController,
    TestEditorController,
};
use crate::editor::TestEditorComponent;
use crate::fault::{FAULT_CONTROLLER_CID, FAULT_PARAM, FAULT_STEP_COUNT, TestFaultComponent};
use crate::gain::GAIN_PARAM;
use crate::instrument::TestInstrumentComponent;
use crate::preset::{TestStatefulComponent, TestStatefulController};
use crate::sdk_enum_i32;
use crate::stateful::STATEFUL_CONTROLLER_CID;
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

    /// The class row for `index` — component + controller pairs in
    /// suite order — or `None` out of range.
    fn class_row(index: i32) -> Option<&'static ClassRow> {
        usize::try_from(index)
            .ok()
            .and_then(|index| CLASS_ROWS.get(index))
    }
}

/// One suite class row — the facts every `PClassInfo*` variant
/// reports for the class.
struct ClassRow {
    cid: TUID,
    /// `PClassInfo::category` — `"Audio Module Class"` or
    /// `"Component Controller Class"`.
    category: &'static str,
    /// Display name (`"test_gain"`, `"test_gain Controller"`, ...).
    name: &'static str,
    /// `PClassInfo2::subCategories` — `""` for controllers.
    sub_categories: &'static str,
}

/// `test_spawn_fail`'s class id — deliberately absent from
/// `createInstance`'s dispatch, so creation always returns
/// `kInvalidArgument` while the class stays enumerable.
const SPAWN_FAIL_CID: TUID = vst3::uid(0x5E2A_9F14, 0x7C3D_8B61, 0xA4E9_1F27, 0xD8B3_6C05);

/// Classes 0/1 = `test_gain`, 2/3 = `test_fault`, 4/5 =
/// `test_editor`, 6/7 = `test_stateful`, 8/9 = `test_instrument` —
/// component + controller pairs in suite order — plus class 10 =
/// `test_spawn_fail`, a listed-but-uncreatable class whose
/// `createInstance` always declines (the spawn-phase
/// `CreateInstance`-rejection oracle).
const CLASS_ROWS: [ClassRow; 11] = [
    ClassRow {
        cid: TestGainComponent::CID,
        category: "Audio Module Class",
        name: "test_gain",
        sub_categories: "Fx",
    },
    ClassRow {
        cid: GAIN_CONTROLLER_CID,
        category: "Component Controller Class",
        name: "test_gain Controller",
        sub_categories: "",
    },
    ClassRow {
        cid: TestFaultComponent::CID,
        category: "Audio Module Class",
        name: "test_fault",
        sub_categories: "Fx",
    },
    ClassRow {
        cid: FAULT_CONTROLLER_CID,
        category: "Component Controller Class",
        name: "test_fault Controller",
        sub_categories: "",
    },
    ClassRow {
        cid: TestEditorComponent::CID,
        category: "Audio Module Class",
        name: "test_editor",
        sub_categories: "Fx",
    },
    ClassRow {
        cid: EDITOR_CONTROLLER_CID,
        category: "Component Controller Class",
        name: "test_editor Controller",
        sub_categories: "",
    },
    ClassRow {
        cid: TestStatefulComponent::CID,
        category: "Audio Module Class",
        name: "test_stateful",
        sub_categories: "Fx",
    },
    ClassRow {
        cid: STATEFUL_CONTROLLER_CID,
        category: "Component Controller Class",
        name: "test_stateful Controller",
        sub_categories: "",
    },
    ClassRow {
        cid: TestInstrumentComponent::CID,
        category: "Audio Module Class",
        name: "test_instrument",
        sub_categories: "Instrument|Fx",
    },
    ClassRow {
        cid: INSTRUMENT_CONTROLLER_CID,
        category: "Component Controller Class",
        name: "test_instrument Controller",
        sub_categories: "",
    },
    ClassRow {
        cid: SPAWN_FAIL_CID,
        category: "Audio Module Class",
        name: "test_spawn_fail",
        sub_categories: "Fx",
    },
];

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
        11
    }

    #[expect(unsafe_code)]
    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        let Some(row) = Self::class_row(index) else {
            return kInvalidArgument;
        };
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.cid = row.cid;
        info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
        copy_cstring(row.category, &mut info.category);
        copy_cstring(row.name, &mut info.name);
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
            GAIN_CONTROLLER_CID | INSTRUMENT_CONTROLLER_CID => {
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
            TestStatefulComponent::CID => {
                ComWrapper::new(TestStatefulComponent::new()).to_com_ptr::<FUnknown>()
            }
            STATEFUL_CONTROLLER_CID => {
                ComWrapper::new(TestStatefulController::new()).to_com_ptr::<FUnknown>()
            }
            TestInstrumentComponent::CID => {
                ComWrapper::new(TestInstrumentComponent::new()).to_com_ptr::<FUnknown>()
            }
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
        let Some(row) = Self::class_row(index) else {
            return kInvalidArgument;
        };
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.cid = row.cid;
        info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
        copy_cstring(row.category, &mut info.category);
        copy_cstring(row.name, &mut info.name);
        info.classFlags = 0;
        copy_cstring(row.sub_categories, &mut info.subCategories);
        copy_cstring("Ruviform Test", &mut info.vendor);
        copy_cstring("0.0.0", &mut info.version);
        copy_cstring("VST 3.x", &mut info.sdkVersion);
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
        let Some(row) = Self::class_row(index) else {
            return kInvalidArgument;
        };
        // SAFETY: caller-provided out-param, checked non-null.
        let info = unsafe { &mut *info };
        info.cid = row.cid;
        info.cardinality = sdk_enum_i32(PClassInfo_::ClassCardinality_::kManyInstances);
        copy_cstring(row.category, &mut info.category);
        copy_wstring(row.name, &mut info.name);
        info.classFlags = 0;
        copy_cstring(row.sub_categories, &mut info.subCategories);
        copy_wstring("Ruviform Test", &mut info.vendor);
        copy_wstring("0.0.0", &mut info.version);
        copy_wstring("VST 3.x", &mut info.sdkVersion);
        kResultOk
    }

    /// No host-context features needed by the `test_gain` fixture —
    /// the call is acknowledged per the `IPluginFactory3` contract.
    #[expect(unsafe_code)]
    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
