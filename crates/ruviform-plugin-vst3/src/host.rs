//! Host-side VST3 bridge (`feature = "host"`): dlopens a real VST3
//! module in-process via `libloading`, scans its class table through
//! the raw `vst3`-crate COM bindings and maps descriptors, parameters
//! and the component/controller lifecycle onto the neutral
//! [`PluginFactory`]/[`Plugin`] contracts for the L1 out-of-process
//! runner (Issue #114 — `docs/evidence/plugin-l1-evidence.md`).
//!
//! ## Lifecycle mapping (neutral → VST3)
//!
//! | Neutral verb | VST3 calls |
//! | --- | --- |
//! | [`Plugin::init`] | `createInstance`(component CID) → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance`(controller CID) — or QI `IEditController` on the component itself when it reports its own CID (single-component plugin) → `initialize(context)` (once on a shared object) → `setComponentHandler` |
//! | [`PluginProcessor::prepare`] | `IComponent::setActive(true)` → `IAudioProcessor::setupProcessing(ProcessSetup)` — the fixture rejects `setupProcessing` below `Activated` (`component.rs`), so the order is pinned |
//! | [`Plugin::start_processing`] | `IAudioProcessor::setProcessing(true)` |
//! | [`PluginProcessor::process`] | `IAudioProcessor::process(ProcessData)` |
//! | [`Plugin::stop_processing`] | `IAudioProcessor::setProcessing(false)` |
//! | `Drop` | best-effort `setProcessing(false)` → `setActive(false)` → `terminate` on both objects → COM `release` → module exit hook → unmap |
//!
//! Teardown is `Drop`-driven: the neutral `Plugin` surface has no
//! `Deactivate`/`Destroy`/`Shutdown` hook (the export Runner owns
//! those lifecycle transitions itself), so the symmetric VST3
//! teardown runs in `Vst3Plugin::drop`. The fixture rejects
//! `setActive(false)` while `Processing` and `terminate` while
//! `Activated`, so the descent order is pinned, not optional.
//!
//! ## `'static` metadata (bounded intern-once)
//!
//! [`PluginDescriptor`] and [`ParamInfo`] fields are `&'static`, but
//! scanned plugin metadata arrives as runtime `String`/`Vec`. The
//! bridge materializes `'static` storage with `String::leak`/`Vec::leak`
//! **only** at `Vst3Library::load` scan time (descriptors and param
//! tables — `factories()` only reads the cache, so repeated calls
//! neither re-instantiate a throwaway pair nor leak a fresh table).
//! This is the
//! sanctioned exception to plugin-core's no-forged-statics rule: the
//! leak is *bounded* (~5 + N strings per descriptor, 2 per param),
//! *once-per-load*, *process-scoped* (the L1 runner is a short-lived
//! child process reclaimed on exit), and *feature-sealed* inside
//! `host`. `descriptors()`/`descriptor()`/`params()` only ever return
//! these cached values — no per-call growth.
//!
//! ## COM host objects
//!
//! The bridge implements its minimal host objects with
//! `Class`/`ComWrapper` — the same pattern as `component.rs`'s test
//! `TestQueue`/`TestChanges`:
//!
//! - `BridgeHostApplication` — `IHostApplication`; supplies the
//!   `initialize` context. `getName` reports the bridge identity;
//!   `createInstance` serves the standard host classes the spec
//!   assigns to it (`IMessage` — required by `IConnectionPoint`
//!   notify — and `IAttributeList`) and declines others honestly.
//! - `BridgeMessage`/`BridgeAttributeList` — the `IMessage`/
//!   `IAttributeList` host classes `createInstance` serves.
//! - `BridgePlugFrame` — `IPlugFrame` handed to the view's `setFrame`;
//!   `resizeView` is a defined refusal (`kResultFalse`).
//! - `BridgeComponentHandler` — `IComponentHandler` +
//!   `IComponentHandler2`, injected via `setComponentHandler`: the
//!   plugin→host dirty-notification surface (Issue #891).
//!   `performEdit` → `DirtyNotice::ParamEdited`,
//!   `setDirty(bool)` → `StateDirty`/`StateClean` and
//!   `restartComponent(kParamValuesChanged)` → `StateDirty` all
//!   enqueue into the instance-local [`DirtyNoticeQueue`] that
//!   `drain_dirty_notices` empties on the host's pull cadence.
//!   `beginEdit`/`endEdit` still answer `kResultOk` (a refused edit
//!   could wedge the plugin's UI thread); `restartComponent` with
//!   any other flag bit, `requestOpenEditor` and the group-edit pair
//!   answer `kNotImplemented` — `kResultOk` there would promise a
//!   resend or surface the bridge never performs.
//! - `BridgeParamQueue` / `BridgeParameterChanges` —
//!   `IParamValueQueue`/`IParameterChanges` supplied as
//!   `ProcessData.inputParameterChanges`. One queue per *declared*
//!   param is built at `prepare` (with reserved point capacity), so
//!   `process` only `clear()`s and `push()`es inside existing
//!   capacity — no allocation on the RT path.
//!
//! ## RT path (`process`)
//!
//! `process` performs no allocation: every buffer the VST3 call needs
//! is pre-sized in `prepare`.
//!
//! - **Input bounce** — `HostInput` yields `&[f32]` per channel, but
//!   `AudioBusBuffers.channelBuffers32` is `*mut *mut Sample32`
//!   (a plugin may write its input in place). Reinterpreting the
//!   immutable host slices as `*mut` is rejected: the bridge copies
//!   host input into owned scratch (`channels × max_frames`,
//!   allocated at `prepare`) and hands the plugin the scratch. An
//!   unconnected input channel delivers silence.
//! - **Output wiring** — `HostOutput.channel_mut` yields a real
//!   `&mut [f32]`, so `channelBuffers32` points directly at the host
//!   channels (the task's explicitly permitted alias). A channel the
//!   host did not wire points at owned scrap and is dropped by
//!   definition.
//! - **Buses** — exactly one audio input bus and one audio output bus
//!   (the v1 defined boundary), `kSample32` only, `kRealtime`.
//! - **Events** — `EventKind::ParamValue` (already normalized)
//!   becomes a `(sample_offset, value)` point on the matching
//!   param's queue; `EventKind::Midi` translates through
//!   [`crate::midi::outgoing_event`] into the preallocated
//!   `IEventList` handed as `ProcessData.inputEvents` (Issue #253 —
//!   note on/off/poly-pressure → VST3 note events, CC/channel
//!   pressure/pitch bend/program change → `kLegacyMIDICCOutEvent`,
//!   `group != 0` and other defined skips counted in `midi_stats`).
//!   Foreign ids, out-of-block offsets and events past
//!   `MAX_EVENTS_PER_BLOCK` are dropped by definition.
//! - **`processContext`** — mapped from the neutral `TransportInfo`
//!   snapshot (`kPlaying`, `projectTimeSamples`, PPQ musical fields)
//!   or null when the host supplies none. `continousTimeSamples` is
//!   left unset: `position()` is the transport playhead, not
//!   engine-continuous time, so the `kContTimeValid` flag is never
//!   raised and the field stays zero.
//!
//! ## Library lifetime (unload ordering)
//!
//! `LibraryInner` (shared via `Arc`) owns the dlopen handle, the
//! module's `IPluginFactory` and the optional exit hook. Factories and
//! instances clone the `Arc`, so the module stays mapped until the
//! last derived COM object is gone: `Vst3Plugin`'s field order drops
//! every `ComPtr` (each `release`s through the mapped vtables) before
//! the `Arc`, and `LibraryInner::drop` releases the factory, runs the
//! platform exit hook, then unmaps — in that order.
//!
//! ## Defined boundaries (v1)
//!
//! - Load/scan/instantiate/lifecycle failures converge to
//!   `Err(Vst3HostError)`/`Err(HostError)`/`Err(PrepareError)` — never
//!   a panic.
//! - `save_state`/`restore_state` drive the standard state verbs
//!   through a bridge-owned `IBStream` (`BridgeStream`): save calls
//!   `IComponent::getState` (+ `IEditController::getState` for
//!   distributed plugins) and frames the two payloads as
//!   `[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl]`; restore runs
//!   the VST3-documented sequence `component.setState` →
//!   `component.getState` → `controller.setComponentState` →
//!   `controller.setState`. A plugin answering `kNotImplemented`
//!   maps to `Err(HostError::Unsupported)` (honest capability
//!   decline); other failures and corrupt adapter frames map to
//!   `Err(HostError::PluginFailed)`. Blobs shorter than the 8-byte
//!   frame header cannot be this adapter's serialization, so they
//!   are foreign state delegated to the plugin unchanged — the
//!   plugin owns its state format and its decline still surfaces as
//!   `Unsupported`. Single-component plugins skip the controller
//!   calls (`ctrl_len = 0`). These are non-RT verbs — the stream's
//!   `Vec`/`RefCell` state exists only inside the call.
//! - `open_gui`/`close_gui` are implemented for Windows
//!   (`kPlatformTypeHWND`): `createView("editor")` →
//!   `isPlatformTypeSupported` → `setFrame(BridgePlugFrame)` →
//!   `attached(parent)`. Other platforms keep the honest
//!   `Err(HostError::Unsupported)` (design
//!   `docs/designs/issue-389-vst3-l0-editor-attach.md` D4).
//! - `create_sendable` keeps its `None` default — `Vst3Plugin` holds
//!   raw channel pointers (`!Send`), so no `Send` instance can be
//!   honestly produced.
//! - A block larger than the negotiated `max_frames`, or `process`
//!   outside the started state, answers `ProcessStatus::Error`.
//! - `IComponent::activateBus`/`IAudioProcessor::setBusArrangements`
//!   are never called — the bridge drives the plugin's *default-active*
//!   bus layout only; a plugin requiring non-default bus state is
//!   outside the v1 boundary.
//! - `ProcessData.outputParameterChanges`/`outputEvents` are always
//!   null — plugin-emitted parameter changes and plugin→host events
//!   are unsupported in v1. `inputEvents` carries the translated
//!   canonical MIDI stream (Issue #253); its defined skips are
//!   counted in `Vst3Plugin::midi_stats`.
//! - Plugin-reported enumeration counts (`countClasses`,
//!   `getParameterCount`, `BusInfo.channelCount`) are clamped to
//!   `MAX_SCAN_*` caps before they
//!   can size an allocation or a scan loop, and `prepare` rejects a
//!   negotiated `max_frames` above the wire protocol's
//!   `WIRE_MAX_FRAMES` ceiling — scratch is sized from the negotiated
//!   bound.

// The generated bindings declare enum constants as `DefaultEnumType`
// (`c_int` on Windows, `c_uint` elsewhere) while struct fields use
// `int32`/`uint32`; the `as` casts below are required on one platform
// and a no-op on the other — suppress `unnecessary_cast` for this
// file, the same resolution as `factory.rs`/`component.rs`.
#![allow(clippy::unnecessary_cast)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::fmt;
use std::path::Path;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

use libloading::Library;
use ruviform_media_core::{Rounding, TimeBase, TransportStatus};
use ruviform_plugin_core::wire::{WIRE_MAX_FRAMES, WIRE_MAX_STATE_BYTES};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, DirtyNotice, DirtyNoticeQueue, ErasedPlugin, Event, EventKind,
    EventList, GuiFocusTarget, GuiHandle, GuiKeyDirection, GuiKeyEvent, GuiModifiers, GuiPopupKind,
    GuiRequest, GuiRequestSink, GuiRequestVerdict, GuiSize, HostError, ParamEvent, ParamFlags,
    ParamId, ParamInfo, ParentLease, Plugin, PluginDescriptor, PluginFactory, PluginProcessor,
    PrepareError, ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo,
    erase,
};
use vst3::Steinberg::IBStream_::IStreamSeekMode_::{kIBSeekCur, kIBSeekEnd, kIBSeekSet};
use vst3::Steinberg::KeyModifier_::{kAlternateKey, kCommandKey, kControlKey, kShiftKey};
use vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0, BusDirection, BusDirections_, BusInfo, Chord,
    Event as VstEvent, FrameRate, IAttributeList, IAttributeList_iid, IAttributeListTrait,
    IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentHandler2,
    IComponentHandler2Trait, IComponentHandlerTrait, IComponentTrait, IConnectionPoint,
    IConnectionPointTrait, IEditController, IEditControllerTrait, IEventList, IHostApplication,
    IHostApplicationTrait, IMessage, IMessage_iid, IMessageTrait, IMidiMapping, IParamValueQueue,
    IParamValueQueueTrait, IParameterChanges, IParameterChangesTrait, MediaTypes_, ParamID,
    ParamValue, ParameterInfo, ParameterInfo_::ParameterFlags_,
    ProcessContext as VstProcessContext, ProcessContext_, ProcessData, ProcessModes_, ProcessSetup,
    RestartFlags_, String128, SymbolicSampleSizes_, TChar, ViewType,
};
use vst3::Steinberg::{
    FUnknown, FUnknown_iid, IBStream, IBStreamTrait, IPlugFrame, IPlugFrameTrait, IPlugView,
    IPlugViewTrait, IPluginBaseTrait, IPluginFactory, IPluginFactory2, IPluginFactory2Trait,
    IPluginFactory3, IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo2,
    PClassInfoW, TBool, TUID, ViewRect, char8, kInvalidArgument, kNotImplemented, kResultFalse,
    kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComPtr, ComRef, ComWrapper, Interface};

use crate::block;
use crate::editor_ops::{
    IRvfEditorOps, IRvfEditorOpsTrait, POPUP_KIND_CHILD_WINDOW, POPUP_KIND_DIALOG,
};
use crate::midi::{self, BridgeEventList, MidiAssignmentTable, MidiBridgeStats};
use crate::sdk_enum_i32;
use crate::strings::copy_wstring;

/// Upper bound of param events forwarded per block — the same bound
/// as the plugin-side adapter's `MAX_BLOCK_EVENTS` scratch. Each
/// preallocated queue reserves this capacity once at `prepare`, so
/// `process` only clears and pushes inside it; excess events are
/// dropped by definition rather than growing on the RT path.
const MAX_EVENTS_PER_BLOCK: usize = 1_024;

/// The only `PClassInfo::category` value the scan adopts — VST3's
/// audio-module class. `"Component Controller Class"` entries are the
/// paired edit controllers, not standalone plugins, and are filtered
/// out.
const AUDIO_MODULE_CATEGORY: &str = "Audio Module Class";

/// Upper bound on plugin-reported scan counts — the same
/// `MAX_SCAN_*` convention as the CLAP host bridge: a hostile or
/// broken module may claim any `countClasses`/`getParameterCount`, so
/// the count is clamped before it can size a `Vec` reservation or a
/// scan loop. The ceilings sit far beyond any sane plugin.
const MAX_SCAN_CLASSES: i32 = 1_024;
/// See [`MAX_SCAN_CLASSES`].
const MAX_SCAN_PARAMS: i32 = 4_096;
/// See [`MAX_SCAN_CLASSES`] — a `BusInfo.channelCount` beyond this is
/// truncated; the channel count sizes the per-channel scratch.
const MAX_SCAN_CHANNELS: usize = 64;

/// Fallback channel count when a component cannot describe its audio
/// buses — the fixture's declared stereo pair.
const CHANNELS_PER_BUS: usize = 2;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why loading or driving a VST3 module through the bridge failed.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — load/scan/instantiate errors.
#[non_exhaustive]
#[derive(Debug)]
pub enum Vst3HostError {
    /// `dlopen`/symbol resolution failed — carries the underlying
    /// `libloading` error verbatim for diagnostics.
    Library(libloading::Error),
    /// An exported module-entry hook (`InitDll`/`ModuleEntry`/
    /// `bundleEntry`) declined initialization.
    ModuleInit,
    /// A required module export is missing — a non-VST3 or broken
    /// binary. Carries the symbol name.
    MissingEntryPoint(&'static str),
    /// `GetPluginFactory` returned a null `IPluginFactory` pointer.
    NullFactory,
    /// A COM vtable call returned a non-`kResultOk` `tresult` —
    /// carries the raw result code.
    Com(tresult),
}

impl fmt::Display for Vst3HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => write!(f, "VST3 library load failed: {error}"),
            Self::ModuleInit => f.write_str("module entry hook declined initialization"),
            Self::MissingEntryPoint(symbol) => {
                write!(f, "module does not export {symbol}")
            }
            Self::NullFactory => f.write_str("GetPluginFactory returned a null pointer"),
            Self::Com(result) => write!(f, "VST3 COM call failed: tresult {result}"),
        }
    }
}

impl std::error::Error for Vst3HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Bounded intern-once helpers + string conversions
// ---------------------------------------------------------------------------

/// Materializes a `&'static str` for one scanned string — see the
/// module-level intern-once policy for why this leak is sanctioned.
/// Empty input returns the `'static` literal without leaking.
fn intern_str(text: &str) -> &'static str {
    if text.is_empty() {
        ""
    } else {
        String::leak(text.to_owned())
    }
}

/// Reads a NUL-terminated `char8` field (VST3 class-info strings) —
/// lossy UTF-8, bounded at the buffer length.
fn c8_string(buf: &[char8]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let bytes: Vec<u8> = buf[..end].iter().map(|c| c.to_ne_bytes()[0]).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Reads a NUL-terminated `TChar` (UTF-16) field (`PClassInfoW`
/// name/vendor/version, `ParameterInfo` title/units) — lossy, bounded
/// at the buffer length.
fn wstring_to_string(buf: &[TChar]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// The descriptor `id` form: the raw `TUID` bytes (the `vst3::uid`
/// platform-order layout, *not* a textual GUID) as 16-byte lowercase
/// hexadecimal — 32 chars.
fn tuid_hex(tuid: &TUID) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(32);
    for &byte in tuid {
        let byte = byte.to_ne_bytes()[0];
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    out
}

/// One scanned class, normalized out of the three `PClassInfo*`
/// spellings (`getClassInfoUnicode` > `getClassInfo2` > `getClassInfo`
/// — the first the factory answers wins).
struct ClassInfo {
    cid: TUID,
    category: String,
    name: String,
    vendor: String,
    version: String,
    /// Raw `subCategories` field — a `|`-separated list.
    sub_categories: String,
}

impl ClassInfo {
    fn from_w(info: &PClassInfoW) -> Self {
        Self {
            cid: info.cid,
            category: c8_string(&info.category),
            name: wstring_to_string(&info.name),
            vendor: wstring_to_string(&info.vendor),
            version: wstring_to_string(&info.version),
            sub_categories: c8_string(&info.subCategories),
        }
    }

    fn from_v2(info: &PClassInfo2) -> Self {
        Self {
            cid: info.cid,
            category: c8_string(&info.category),
            name: c8_string(&info.name),
            vendor: c8_string(&info.vendor),
            version: c8_string(&info.version),
            sub_categories: c8_string(&info.subCategories),
        }
    }

    /// The base `PClassInfo` carries no vendor/version/subcategories —
    /// honest empty strings.
    fn from_v1(info: &PClassInfo) -> Self {
        Self {
            cid: info.cid,
            category: c8_string(&info.category),
            name: c8_string(&info.name),
            vendor: String::new(),
            version: String::new(),
            sub_categories: String::new(),
        }
    }

    /// Maps the scanned class onto the neutral descriptor, interning
    /// every field once; `features` keep their declared `|` order.
    fn descriptor(&self) -> PluginDescriptor {
        let features: Vec<&'static str> = self
            .sub_categories
            .split('|')
            .filter(|feature| !feature.is_empty())
            .map(intern_str)
            .collect();
        PluginDescriptor {
            id: intern_str(&tuid_hex(&self.cid)),
            name: intern_str(&self.name),
            vendor: intern_str(&self.vendor),
            version: intern_str(&self.version),
            features: if features.is_empty() {
                &[]
            } else {
                features.leak()
            },
        }
    }
}

// ---------------------------------------------------------------------------
// COM host objects
// ---------------------------------------------------------------------------

/// Minimal `IAttributeList` — per-`AttrID` typed stores. The VST3
/// contract: `set*` stores (replaces), `get*` on a missing id answers
/// `kResultFalse`, null out-args answer `kInvalidArgument`.
#[derive(Default)]
struct BridgeAttributeList {
    ints: RefCell<HashMap<Vec<u8>, i64>>,
    floats: RefCell<HashMap<Vec<u8>, f64>>,
    strings: RefCell<HashMap<Vec<u8>, Vec<u16>>>,
    binaries: RefCell<HashMap<Vec<u8>, Box<[u8]>>>,
}

/// Reads an `AttrID` (`FIDString`) into owned bytes — the map key.
/// Null/empty ids are rejected by the callers' guard.
///
/// # Safety
/// `id` must point at a valid NUL-terminated `char8` string.
#[expect(unsafe_code)]
unsafe fn attr_key(id: *const c_char) -> Vec<u8> {
    // SAFETY: caller guarantees a valid NUL-terminated string.
    unsafe { std::ffi::CStr::from_ptr(id) }.to_bytes().to_vec()
}

impl Class for BridgeAttributeList {
    type Interfaces = (IAttributeList,);
}

#[expect(unsafe_code)]
impl IAttributeListTrait for BridgeAttributeList {
    unsafe fn setInt(&self, id: *const c_char, value: i64) -> tresult {
        if id.is_null() {
            return kInvalidArgument;
        }
        self.ints
            .borrow_mut()
            .insert(unsafe { attr_key(id) }, value);
        kResultOk
    }

    unsafe fn getInt(&self, id: *const c_char, value: *mut i64) -> tresult {
        if id.is_null() || value.is_null() {
            return kInvalidArgument;
        }
        match self.ints.borrow().get(&unsafe { attr_key(id) }) {
            Some(v) => {
                // SAFETY: `value` is a valid out-ptr, checked above.
                unsafe { *value = *v };
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn setFloat(&self, id: *const c_char, value: f64) -> tresult {
        if id.is_null() {
            return kInvalidArgument;
        }
        self.floats
            .borrow_mut()
            .insert(unsafe { attr_key(id) }, value);
        kResultOk
    }

    unsafe fn getFloat(&self, id: *const c_char, value: *mut f64) -> tresult {
        if id.is_null() || value.is_null() {
            return kInvalidArgument;
        }
        match self.floats.borrow().get(&unsafe { attr_key(id) }) {
            Some(v) => {
                // SAFETY: `value` is a valid out-ptr, checked above.
                unsafe { *value = *v };
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn setString(&self, id: *const c_char, string: *const TChar) -> tresult {
        if id.is_null() || string.is_null() {
            return kInvalidArgument;
        }
        // `TChar` is UTF-16 on Windows — copy up to the NUL.
        let mut len = 0usize;
        // SAFETY: `string` is a valid NUL-terminated TChar buffer.
        while unsafe { *string.add(len) } != 0 {
            len += 1;
        }
        // SAFETY: `len` TChar elements are readable (checked above).
        let data = unsafe { std::slice::from_raw_parts(string, len) }.to_vec();
        self.strings
            .borrow_mut()
            .insert(unsafe { attr_key(id) }, data);
        kResultOk
    }

    unsafe fn getString(&self, id: *const c_char, string: *mut TChar, size_bytes: u32) -> tresult {
        // `sizeInBytes` must hold at least one `TChar` — the NUL
        // terminator write needs 2 bytes, so sub-`TChar` buffers are
        // rejected rather than truncated into (plugin-triggerable OOB).
        if id.is_null() || string.is_null() || (size_bytes as usize) < size_of::<TChar>() {
            return kInvalidArgument;
        }
        match self.strings.borrow().get(&unsafe { attr_key(id) }) {
            Some(data) => {
                // `sizeInBytes` counts bytes; `TChar` is 2 bytes.
                let capacity = (size_bytes as usize / 2).saturating_sub(1);
                let write_len = data.len().min(capacity);
                // SAFETY: `string` has room for `size_bytes` bytes per
                // the contract; we stay strictly inside it.
                unsafe {
                    ptr::copy_nonoverlapping(data.as_ptr(), string, write_len);
                    *string.add(write_len) = 0;
                }
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn setBinary(&self, id: *const c_char, data: *const c_void, size_bytes: u32) -> tresult {
        if id.is_null() || (data.is_null() && size_bytes > 0) {
            return kInvalidArgument;
        }
        // `from_raw_parts` requires a valid non-null ptr even at len
        // 0 — handle the empty binary without dereferencing `data`.
        let bytes: Box<[u8]> = if size_bytes == 0 {
            Box::new([])
        } else {
            // SAFETY: `data..data+size_bytes` is readable per contract
            // (non-null checked above).
            unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size_bytes as usize) }
                .to_vec()
                .into_boxed_slice()
        };
        self.binaries
            .borrow_mut()
            .insert(unsafe { attr_key(id) }, bytes);
        kResultOk
    }

    unsafe fn getBinary(
        &self,
        id: *const c_char,
        data: *mut *const c_void,
        size_bytes: *mut u32,
    ) -> tresult {
        if id.is_null() || data.is_null() || size_bytes.is_null() {
            return kInvalidArgument;
        }
        match self.binaries.borrow().get(&unsafe { attr_key(id) }) {
            Some(bytes) => {
                // The returned pointer borrows our stored bytes — the
                // contract's own sharing rule (caller does not free).
                // SAFETY: out-ptrs checked above.
                unsafe {
                    *data = bytes.as_ptr().cast::<c_void>();
                    *size_bytes = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                }
                kResultOk
            }
            None => kResultFalse,
        }
    }
}

/// Minimal `IMessage` — the host-class plugins create through
/// `IHostApplication::createInstance` (e.g. to send the
/// `IConnectionPoint` notification). Attributes live in a real
/// `BridgeAttributeList` the message owns.
struct BridgeMessage {
    id: RefCell<Vec<u8>>,
    attributes: ComPtr<IAttributeList>,
}

impl BridgeMessage {
    fn new() -> Self {
        let attrs = ComWrapper::new(BridgeAttributeList::default());
        Self {
            id: RefCell::new(Vec::new()),
            attributes: attrs
                .to_com_ptr::<IAttributeList>()
                .expect("BridgeAttributeList exposes IAttributeList"),
        }
    }
}

impl Class for BridgeMessage {
    type Interfaces = (IMessage,);
}

#[expect(unsafe_code)]
impl IMessageTrait for BridgeMessage {
    unsafe fn getMessageID(&self) -> *const char8 {
        let id = self.id.borrow();
        if id.is_empty() {
            ptr::null()
        } else {
            id.as_ptr().cast()
        }
    }

    unsafe fn setMessageID(&self, id: *const char8) {
        *self.id.borrow_mut() = if id.is_null() {
            Vec::new()
        } else {
            // SAFETY: `id` is a valid NUL-terminated FIDString.
            let mut v = unsafe { std::ffi::CStr::from_ptr(id.cast()) }
                .to_bytes()
                .to_vec();
            v.push(0);
            v
        };
    }

    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes.as_ptr()
    }
}

/// Minimal `IHostApplication` — supplies the `initialize` context;
/// `getName` reports the bridge identity. `createInstance` serves the
/// standard host classes the VST3 spec assigns to it (`IMessage`,
/// `IAttributeList`) — other CIDs answer `kNotImplemented` honestly.
struct BridgeHostApplication;

impl Class for BridgeHostApplication {
    type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for BridgeHostApplication {
    #[expect(unsafe_code)]
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        if name.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: `name` is a valid `String128` out-param (128 `TChar`s
        // per the VST3 contract), checked non-null.
        copy_wstring("Ruviform VST3 Host", unsafe { &mut *name });
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn createInstance(
        &self,
        cid: *mut TUID,
        iid: *mut TUID,
        obj: *mut *mut c_void,
    ) -> tresult {
        if cid.is_null() || iid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: `obj` is a valid out-ptr — null it so a declined
        // request never hands back garbage.
        unsafe { *obj = ptr::null_mut() };
        // The VST3 spec assigns these standard classes to the host's
        // `createInstance`: `IMessage` (used by `IConnectionPoint`
        // notify — its absence crashes connection-based plugins) and
        // `IAttributeList` (message attributes + presets).
        // SAFETY: `cid`/`iid` are valid 16-byte TUIDs.
        let (cid, requested) = unsafe { (*cid, *iid) };
        if cid == IMessage_iid {
            // Honour `iid` — a mismatched request must not hand back
            // a mistyped pointer.
            if requested != IMessage_iid && requested != FUnknown_iid {
                return kInvalidArgument;
            }
            return match ComWrapper::new(BridgeMessage::new()).to_com_ptr::<IMessage>() {
                Some(p) => {
                    // SAFETY: `obj` checked; `into_raw` hands the
                    // ownership to the caller per the COM out contract.
                    unsafe { *obj = p.into_raw().cast::<c_void>() };
                    kResultOk
                }
                None => kResultFalse,
            };
        }
        if cid == IAttributeList_iid {
            if requested != IAttributeList_iid && requested != FUnknown_iid {
                return kInvalidArgument;
            }
            return match ComWrapper::new(BridgeAttributeList::default())
                .to_com_ptr::<IAttributeList>()
            {
                Some(p) => {
                    // SAFETY: same ownership hand-off as above.
                    unsafe { *obj = p.into_raw().cast::<c_void>() };
                    kResultOk
                }
                None => kResultFalse,
            };
        }
        kNotImplemented
    }
}

/// Minimal `IPlugFrame` — the host frame handed to `setFrame` before
/// `attached`, per-instance so `resizeView` can reach the request
/// route the session installed (Issue #873). `resizeView` is the
/// plugin-initiated resize request: with a live route it delivers
/// [`GuiRequest::Resize`] through the shared sink and answers the
/// arbiter's verdict — `Accepted` runs the `onSize` roundtrip on the
/// calling thread before `kResultOk`; `Refused` and every unrouted
/// state keep the honest `kResultFalse` (container geometry stays
/// with the slot side, `NativeSurfaceSlot`/`set_geometry` —
/// `docs/designs/issue-389-vst3-l0-editor-attach.md` D2, updated by
/// `docs/designs/issue-873-plugin-gui-request.md`).
struct BridgePlugFrame {
    /// Shared cell mirroring `Vst3Plugin`'s request-sink slot — the
    /// session installs `Some` at open and detaches at close; `None`
    /// inside keeps `resizeView` at the defined refusal.
    request_sink: Rc<RefCell<Option<Box<GuiRequestSink>>>>,
}

impl Class for BridgePlugFrame {
    type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for BridgePlugFrame {
    /// `resizeView` → request sink → verdict → `onSize` roundtrip.
    /// The plugin proposed the size itself, so `canResize`/
    /// `checkSizeConstraint` are not re-run — the container-side
    /// correction, if any, is the arbiter's `granted`.
    #[expect(unsafe_code)]
    #[expect(clippy::cast_possible_truncation)]
    unsafe fn resizeView(&self, view: *mut IPlugView, new_size: *mut ViewRect) -> tresult {
        if view.is_null() || new_size.is_null() {
            return kInvalidArgument;
        }
        // `try_borrow_mut` — a re-entrant request (e.g. a plugin
        // calling `resizeView` from inside `onSize`) gets the defined
        // refusal, never a `RefCell` panic across the ABI.
        let Ok(mut slot) = self.request_sink.try_borrow_mut() else {
            return kResultFalse;
        };
        let Some(sink) = slot.as_mut() else {
            return kResultFalse;
        };
        // SAFETY: `new_size` checked non-null; `ViewRect` is a POD the
        // plugin owns for the call.
        let rect = unsafe { *new_size };
        let verdict = sink(GuiRequest::Resize {
            size: GuiSize {
                width: f64::from(rect.right - rect.left),
                height: f64::from(rect.bottom - rect.top),
            },
        });
        // Release the borrow before re-entering the plugin through
        // `onSize` — it may legitimately issue a nested request.
        drop(slot);
        let GuiRequestVerdict::Accepted { granted } = verdict else {
            return kResultFalse;
        };
        let applied = granted.unwrap_or(GuiSize {
            width: f64::from(rect.right - rect.left),
            height: f64::from(rect.bottom - rect.top),
        });
        let mut applied_rect = ViewRect {
            left: 0,
            top: 0,
            right: applied.width.round() as i32,
            bottom: applied.height.round() as i32,
        };
        // SAFETY: `view` checked non-null — `ComRef` borrows the
        // plugin's `IPlugView` without addref for the call's
        // duration; `IPlugView` calls run on the caller's UI thread
        // per the open_gui thread contract. `onSize` carries the
        // final (possibly corrected) size — the plugin's honest
        // answer is returned verbatim.
        let Some(view_ref) = (unsafe { ComRef::<IPlugView>::from_raw(view) }) else {
            return kResultFalse;
        };
        unsafe { view_ref.onSize(&raw mut applied_rect) }
    }
}

/// `IComponentHandler` + `IComponentHandler2` — the plugin→host
/// dirty-notification surface (Issue #891). `performEdit`,
/// `setDirty` and `restartComponent(kParamValuesChanged)` capture
/// into the instance-local [`DirtyNoticeQueue`] the owning
/// `Vst3Plugin` drains through `drain_dirty_notices` — the
/// acknowledge-and-drop boundary of the v1 stub is gone: every
/// dirty-relevant call now lands on the neutral pull surface.
///
/// `beginEdit`/`endEdit` still answer `kResultOk` — gesture edges
/// carry no payload the neutral vocabulary models (a refused edge
/// could wedge a plugin that gates its UI on the ack), and
/// `performEdit` inside the gesture already delivers the meaningful
/// content.
struct BridgeComponentHandler {
    /// The instance's pending dirty-notification queue — shared with
    /// the owning `Vst3Plugin` (`Rc`/`RefCell`: every VST3 handler
    /// call runs on the single runner command thread the whole
    /// bridge lives on).
    notices: Rc<RefCell<DirtyNoticeQueue>>,
}

impl BridgeComponentHandler {
    /// A handler enqueueing into `notices` — the owning plugin
    /// passes its `dirty_notices` cell.
    fn new(notices: Rc<RefCell<DirtyNoticeQueue>>) -> Self {
        Self { notices }
    }

    /// Best-effort enqueue — a `borrow_mut` conflict means a
    /// re-entrant call while the queue is mid-drain on this same
    /// thread, which the VST3 contract does not produce; the defined
    /// drop keeps the answer honest rather than deadlocking.
    fn enqueue(&self, notice: DirtyNotice) {
        if let Ok(mut queue) = self.notices.try_borrow_mut() {
            queue.enqueue(notice);
        }
    }
}

impl Class for BridgeComponentHandler {
    type Interfaces = (IComponentHandler, IComponentHandler2);
}

impl IComponentHandlerTrait for BridgeComponentHandler {
    #[expect(unsafe_code)]
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    /// `performEdit(id, value_normalized)` → `ParamEdited` — the
    /// plugin reports a parameter edit outside host-driven writes.
    /// `ParamValue::new` (plugin-core — the vst3 `ParamValue` alias
    /// above is the `f64` typedef) clamps finite input into `[0,1]`;
    /// a non-finite report cannot carry a value, so it degrades to
    /// the bare [`DirtyNotice::StateDirty`] flag — still an honest
    /// "state diverged", never a silent drop.
    #[expect(unsafe_code)]
    unsafe fn performEdit(&self, id: ParamID, value_normalized: ParamValue) -> tresult {
        let notice = ruviform_plugin_core::ParamValue::new(value_normalized).map_or(
            DirtyNotice::StateDirty,
            |value| DirtyNotice::ParamEdited {
                param_id: ParamId(id),
                value,
            },
        );
        self.enqueue(notice);
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn endEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    /// `kParamValuesChanged` is the one restart bit the bridge
    /// honestly serves: parameter values changed → the state diverged
    /// → latch [`DirtyNotice::StateDirty`] and answer `kResultOk`.
    /// Any other flag (`kReloadComponent`, `kIoChanged`, latency,
    /// titles, …) requests a resend the bridge never performs —
    /// `kNotImplemented` stays the honest refusal (`kResultOk` there
    /// would promise a resend that never lands).
    #[expect(unsafe_code)]
    unsafe fn restartComponent(&self, flags: i32) -> tresult {
        if flags & !RestartFlags_::kParamValuesChanged != 0 {
            return kNotImplemented;
        }
        if flags & RestartFlags_::kParamValuesChanged != 0 {
            self.enqueue(DirtyNotice::StateDirty);
        }
        kResultOk
    }
}

impl IComponentHandler2Trait for BridgeComponentHandler {
    /// `setDirty(bool)` → the flag edge itself — `true` latches
    /// [`DirtyNotice::StateDirty`], `false` latches
    /// [`DirtyNotice::StateClean`] (the plugin declares it re-synced
    /// with the saved state; the queue's last-declaration-wins latch
    /// delivers the final state).
    #[expect(unsafe_code)]
    unsafe fn setDirty(&self, state: TBool) -> tresult {
        self.enqueue(if state != 0 {
            DirtyNotice::StateDirty
        } else {
            DirtyNotice::StateClean
        });
        kResultOk
    }

    /// `requestOpenEditor` — GUI policy is host-driven through
    /// `open_gui`, never plugin-spontaneous in this bridge; the
    /// honest refusal.
    #[expect(unsafe_code)]
    unsafe fn requestOpenEditor(&self, _name: vst3::Steinberg::FIDString) -> tresult {
        kNotImplemented
    }

    /// `startGroupEdit`/`finishGroupEdit` — gesture-group transport
    /// is outside the dirty-notification slice (no group-edit
    /// vocabulary exists to carry it); the honest refusal.
    #[expect(unsafe_code)]
    unsafe fn startGroupEdit(&self) -> tresult {
        kNotImplemented
    }

    /// See [`Self::startGroupEdit`].
    #[expect(unsafe_code)]
    unsafe fn finishGroupEdit(&self) -> tresult {
        kNotImplemented
    }
}

/// Mutable state behind [`BridgeStream`]'s `&self` trait methods —
/// one `RefCell` so `read`/`write`/`seek` never hold split borrows.
struct BridgeStreamState {
    data: Vec<u8>,
    /// Absolute position — `u64` because `tell`/`seek` speak `int64`;
    /// positions are always non-negative (negative targets are
    /// refused before they land here) and may exceed `data.len()`
    /// (file-like: reads hit EOF, writes zero-fill the gap).
    pos: u64,
}

/// Bridge-owned `IBStream` — the byte pipe the standard state verbs
/// (`IComponent::setState`/`getState`,
/// `IEditController::setComponentState`/`setState`/`getState`) read
/// and write through. One instance exists only inside a
/// `save_state`/`restore_state` call — a non-RT verb, so the `Vec`
/// growth, `RefCell` borrows and allocation are confined to the
/// call and never touch the `process` path. The borrow is released
/// before each trait method returns, so a re-entrant FFI call meets
/// `try_borrow`'s defined `kResultFalse`, never a panic across the
/// ABI.
///
/// Writes are bounded by [`WIRE_MAX_STATE_BYTES`] — the same ceiling
/// the wire protocol enforces on the neutral blob. A plugin seeking
/// far forward then writing cannot force an unbounded allocation:
/// the stream answers a defined short write / `kResultFalse`
/// instead (design `docs/designs/issue-630-adapter-state-bridge.md`).
struct BridgeStream {
    state: RefCell<BridgeStreamState>,
}

impl BridgeStream {
    /// An empty stream — the `getState` write target.
    fn new() -> Self {
        Self {
            state: RefCell::new(BridgeStreamState {
                data: Vec::new(),
                pos: 0,
            }),
        }
    }

    /// A stream pre-filled with `bytes` at position 0 — the
    /// `setState`/`setComponentState` read source.
    fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            state: RefCell::new(BridgeStreamState {
                data: bytes,
                pos: 0,
            }),
        }
    }

    /// Drains the accumulated bytes — the stream is single-use
    /// inside a state call, so taking (not cloning) is the honest
    /// read-out once the plugin call has returned.
    fn take_bytes(&self) -> Vec<u8> {
        std::mem::take(&mut self.state.borrow_mut().data)
    }
}

impl Default for BridgeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Class for BridgeStream {
    type Interfaces = (IBStream,);
}

#[expect(unsafe_code)]
impl IBStreamTrait for BridgeStream {
    unsafe fn read(
        &self,
        buffer: *mut c_void,
        num_bytes: i32,
        num_bytes_read: *mut i32,
    ) -> tresult {
        if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
            return kInvalidArgument;
        }
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
        // EOF is honest: fewer bytes than requested is reported
        // through `num_bytes_read`, never fabricated.
        let count = state
            .data
            .len()
            .saturating_sub(start)
            .min(usize::try_from(num_bytes).unwrap_or(0));
        if count > 0 {
            // SAFETY: `buffer` is a valid `num_bytes`-byte destination
            // per the IBStream contract (non-null checked for
            // `num_bytes > 0`); `count <= num_bytes` and the source
            // slice is inside `state.data`.
            unsafe {
                ptr::copy_nonoverlapping(
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
            // SAFETY: `num_bytes_read` is the contract's out-ptr —
            // `count <= i32::MAX` because `num_bytes` is an `i32`.
            unsafe { *num_bytes_read = i32::try_from(count).unwrap_or(i32::MAX) };
        }
        kResultOk
    }

    unsafe fn write(
        &self,
        buffer: *mut c_void,
        num_bytes: i32,
        num_bytes_written: *mut i32,
    ) -> tresult {
        if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
            return kInvalidArgument;
        }
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        let start = usize::try_from(state.pos).unwrap_or(usize::MAX);
        // Bounded growth: a seek far past the end cannot size an
        // unbounded `Vec` — the write is capped at the wire ceiling
        // and the short count is reported honestly.
        let ceiling = usize::try_from(WIRE_MAX_STATE_BYTES).unwrap_or(usize::MAX);
        let room = ceiling.saturating_sub(start);
        let count = room.min(usize::try_from(num_bytes).unwrap_or(0));
        if count > 0 {
            let end = start.saturating_add(count);
            // Zero-fill any seek gap, then extend for the payload —
            // file-like semantics for a forward-seeking plugin.
            state.data.resize(end, 0);
            // SAFETY: `buffer` points at `num_bytes` readable bytes
            // per the IBStream contract (non-null checked); the
            // destination was just sized to hold `count` bytes.
            unsafe {
                ptr::copy_nonoverlapping(
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
        if count < usize::try_from(num_bytes).unwrap_or(0) {
            // Stream-full short write — defined refusal, the caller
            // sees the honest partial count.
            return kResultFalse;
        }
        kResultOk
    }

    unsafe fn seek(&self, pos: i64, mode: i32, result: *mut i64) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
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
        let Ok(state) = self.state.try_borrow() else {
            return kResultFalse;
        };
        if !pos.is_null() {
            // SAFETY: `pos` is the contract's out-ptr.
            unsafe { *pos = i64::try_from(state.pos).unwrap_or(i64::MAX) };
        }
        kResultOk
    }
}

/// One parameter's point list — a bridge-owned `IParamValueQueue`
/// handed to the plugin. `writable` selects the direction: the
/// `inputParameterChanges` set is host-filled and plugin-read
/// (`addPoint` declines), the `outputParameterChanges` set is
/// plugin-filled and host-read. The point `Vec` is allocated at
/// `prepare` with `MAX_EVENTS_PER_BLOCK` capacity; `process` only
/// `clear()`s and `push()`es/`addPoint()`s inside it.
///
/// The `RefCell` exists because the COM trait methods take `&self`;
/// the queue is *call-scoped*: the bridge fills input queues (or
/// collects output queues) inside `process`, the plugin touches them
/// inside the same `process` call, and the single-threaded child
/// contract (the L1 runner drives one process thread) means a borrow
/// can only conflict with a re-entrant FFI call — answered by
/// `try_borrow`'s defined `kResultFalse`, never a cross-thread race
/// or a `RefCell` panic across the ABI.
struct BridgeParamQueue {
    id: u32,
    writable: bool,
    points: RefCell<Vec<(i32, f64)>>,
}

impl Class for BridgeParamQueue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for BridgeParamQueue {
    #[expect(unsafe_code)]
    unsafe fn getParameterId(&self) -> ParamID {
        self.id
    }

    #[expect(unsafe_code)]
    unsafe fn getPointCount(&self) -> i32 {
        self.points
            .try_borrow()
            .map_or(0, |points| i32::try_from(points.len()).unwrap_or(i32::MAX))
    }

    #[expect(unsafe_code)]
    unsafe fn getPoint(&self, index: i32, sample_offset: *mut i32, value: *mut f64) -> tresult {
        if sample_offset.is_null() || value.is_null() {
            return kResultFalse;
        }
        let Ok(points) = self.points.try_borrow() else {
            return kResultFalse;
        };
        let Some(&(offset, point)) = points.get(usize::try_from(index).unwrap_or(usize::MAX))
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

    /// Appends a point — writable (output) queues only; read-only
    /// (input) queues keep declining with `kNotImplemented`. A full
    /// queue, a re-entrant borrow or a null `index` is the defined
    /// `kResultFalse`; the point index written to `*index` is the
    /// appended position.
    #[expect(unsafe_code)]
    unsafe fn addPoint(&self, sample_offset: i32, value: f64, index: *mut i32) -> tresult {
        if !self.writable || index.is_null() {
            return kResultFalse;
        }
        let Ok(mut points) = self.points.try_borrow_mut() else {
            return kResultFalse;
        };
        if points.len() >= MAX_EVENTS_PER_BLOCK {
            return kResultFalse;
        }
        points.push((sample_offset, value));
        // SAFETY: `index` is non-null per the check above.
        unsafe {
            *index = i32::try_from(points.len() - 1).unwrap_or(i32::MAX);
        }
        kResultTrue
    }
}

/// The `IParameterChanges` supplied to `process` — a fixed queue set
/// built at `prepare`; the `ComPtr`s keep the wrapped
/// [`BridgeParamQueue`] objects alive for the instance's life.
/// `writable` marks the `outputParameterChanges` direction: the
/// plugin calls `addParameterData` to fetch-or-decline a queue. Each
/// entry pairs the queue's `ParamID` with its `ComPtr` — the id is
/// bridge-side metadata the COM pointer cannot answer cheaply.
struct BridgeParameterChanges {
    queues: Vec<(u32, ComPtr<IParamValueQueue>)>,
    writable: bool,
}

impl Class for BridgeParameterChanges {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for BridgeParameterChanges {
    #[expect(unsafe_code)]
    unsafe fn getParameterCount(&self) -> i32 {
        i32::try_from(self.queues.len()).unwrap_or(i32::MAX)
    }

    #[expect(unsafe_code)]
    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        self.queues
            .get(usize::try_from(index).unwrap_or(usize::MAX))
            .map_or(ptr::null_mut(), |(_, queue)| queue.as_ptr())
    }

    /// Writable sets resolve `ParamID` → the matching preallocated
    /// queue (the plugin's output-parameter lookup); read-only sets
    /// keep declining — the host never adds points to its own input
    /// queues. An unknown id, a null `id`/`index` or a saturated
    /// queue set is the defined null return.
    #[expect(unsafe_code)]
    unsafe fn addParameterData(
        &self,
        id: *const ParamID,
        index: *mut i32,
    ) -> *mut IParamValueQueue {
        if !self.writable || id.is_null() || index.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: `id` is a valid `ParamID` in-param per the check.
        let id = unsafe { *id };
        let Some(position) = self.queues.iter().position(|(queue_id, _)| *queue_id == id) else {
            return ptr::null_mut();
        };
        let Some((_, queue)) = self.queues.get(position) else {
            return ptr::null_mut();
        };
        // SAFETY: `index` is a valid out-param per the check.
        unsafe {
            *index = i32::try_from(position).unwrap_or(i32::MAX);
        }
        queue.as_ptr()
    }
}

// ---------------------------------------------------------------------------
// Library / descriptor scan
// ---------------------------------------------------------------------------

/// The shared dlopen handle — the `IPluginFactory` COM object, the
/// optional exit hook and the `libloading::Library`, dropped in that
/// order. `Arc`-shared by the library/factories/instances so the
/// module cannot unmap while a derived COM pointer lives (the OFX
/// `OfxLibrary::share` precedent).
struct LibraryInner {
    /// The module's `IPluginFactory` — `Option` so `Drop` can release
    /// it explicitly before the exit hook/unmap.
    factory: Option<ComPtr<IPluginFactory>>,
    /// Optional platform module-exit hook (`ExitDll`/`ModuleExit`/
    /// `bundleExit`) — run while the module is still mapped.
    exit_hook: Option<unsafe extern "system" fn() -> bool>,
    /// The dlopen handle — never read, only dropped: its `Drop` unmaps
    /// the module and therefore must run last, after the factory
    /// release and exit hook above.
    _lib: Library,
}

impl Drop for LibraryInner {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // Release the factory while the module code is still mapped —
        // `release` reaches the module's vtable.
        drop(self.factory.take());
        if let Some(exit) = self.exit_hook {
            // SAFETY: `exit` was resolved from this module's exports;
            // `self.lib` is still mapped (it drops after this body).
            unsafe {
                let _ = exit();
            }
        }
    }
}

/// RAII guard for the load path between `module_entry` and
/// `LibraryInner`: owns the mapped `Library` plus the resolved exit
/// hook so a post-entry failure (missing `GetPluginFactory`, null
/// factory, failed class/param scan) still runs the module's exit
/// hook *before* the unmap — the entry/exit pairing the platform
/// module contract requires. `disarm` transfers both into
/// `LibraryInner`, which carries the same obligation from then on.
struct LoadGuard {
    /// `Option` so `disarm` can move the handle out — a `Drop` type
    /// cannot move fields out of itself.
    lib: Option<Library>,
    /// Resolved eagerly by [`LoadGuard::armed`] — pure symbol lookup,
    /// no module code runs.
    exit_hook: Option<unsafe extern "system" fn() -> bool>,
}

impl LoadGuard {
    /// Wraps `lib` once `module_entry` has succeeded — from here every
    /// early `?`-return is covered by the paired exit hook.
    fn armed(lib: Library) -> Self {
        let exit_hook = module_exit(&lib);
        Self {
            lib: Some(lib),
            exit_hook,
        }
    }

    /// The mapped module, for symbol resolution.
    fn lib(&self) -> &Library {
        self.lib
            .as_ref()
            .expect("LoadGuard holds the library until disarm")
    }

    /// Disarms the guard, yielding the owned parts for `LibraryInner`.
    fn disarm(mut self) -> (Library, Option<unsafe extern "system" fn() -> bool>) {
        (
            self.lib
                .take()
                .expect("LoadGuard holds the library until disarm"),
            self.exit_hook.take(),
        )
    }
}

impl Drop for LoadGuard {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        if let Some(exit) = self.exit_hook {
            // SAFETY: `exit` was resolved from this module's exports;
            // `self.lib` is still mapped (fields drop after this
            // body), so the hook runs before the unmap.
            unsafe {
                let _ = exit();
            }
        }
    }
}

/// A loaded VST3 module: the dlopen handle plus the load-time class
/// scan (interned once — see the module docs).
///
/// `descriptors()`/`factories()` are the public surface for the L1
/// runner; instances (`create()` → [`Plugin`]) keep the module mapped
/// through the shared `Arc`.
pub struct Vst3Library {
    inner: Arc<LibraryInner>,
    /// Cached, interned-once descriptors — `descriptors()` returns
    /// this slice verbatim.
    descriptors: Vec<PluginDescriptor>,
    /// Index-aligned instantiation CID for each descriptor.
    cids: Vec<TUID>,
    /// Index-aligned interned param table per descriptor — enumerated
    /// once at load ([`enumerate_params`]) so `factories()` neither
    /// re-instantiates a throwaway component/controller pair nor
    /// leaks a fresh table per call.
    params_tables: Vec<&'static [ParamInfo]>,
}

impl Vst3Library {
    /// Loads a VST3 module (`dlopen`/`LoadLibrary`), runs the optional
    /// platform entry hook, resolves `GetPluginFactory` and performs
    /// the class + param-table scan. Once the entry hook has run,
    /// `LoadGuard` pairs every failure return with the module's exit
    /// hook before the unmap.
    ///
    /// **RT contract:** non-RT only — dlopen, module init, factory
    /// scan.
    ///
    /// # Errors
    ///
    /// [`Vst3HostError::Library`] when the file cannot be loaded or a
    /// symbol is missing; [`Vst3HostError::ModuleInit`] when an
    /// exported entry hook declines; [`Vst3HostError::MissingEntryPoint`]
    /// when `GetPluginFactory` is absent;
    /// [`Vst3HostError::NullFactory`] when it returns null;
    /// [`Vst3HostError::Com`] when a scan-time COM call fails.
    #[expect(unsafe_code)]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Vst3HostError> {
        // SAFETY: `Library::new` maps an external shared object into
        // this process — loading arbitrary plugin code is the caller's
        // contract (the same boundary as the OFX adapter's
        // `load_library`).
        let lib = unsafe { Library::new(path.as_ref()) }.map_err(Vst3HostError::Library)?;
        module_entry(&lib)?;
        // `module_entry` succeeded — from here every early return must
        // run the paired exit hook before the module unmaps.
        // `LoadGuard` owns that contract until `disarm` hands both to
        // `LibraryInner`.
        let guard = LoadGuard::armed(lib);
        // SAFETY: `get` only resolves the symbol; the call below is
        // the module's declared entry point and returns an owned
        // `IPluginFactory` pointer (or null) whose ownership transfers
        // to the caller.
        let get_factory = unsafe {
            guard
                .lib()
                .get::<unsafe extern "system" fn() -> *mut IPluginFactory>(b"GetPluginFactory\0")
        }
        .map_err(|_| Vst3HostError::MissingEntryPoint("GetPluginFactory"))?;
        // SAFETY: see above — the returned pointer is owned or null.
        let factory =
            unsafe { ComPtr::from_raw(get_factory()) }.ok_or(Vst3HostError::NullFactory)?;
        let (descriptors, cids) = scan_classes(&factory)?;
        // Param tables are enumerated here, once per load, so
        // `factories()` only ever reads the cache.
        let params_tables = cids
            .iter()
            .map(|cid| enumerate_params(&factory, cid))
            .collect();
        let (lib, exit_hook) = guard.disarm();
        let inner = Arc::new(LibraryInner {
            factory: Some(factory),
            exit_hook,
            _lib: lib,
        });
        Ok(Self {
            inner,
            descriptors,
            cids,
            params_tables,
        })
    }

    /// The descriptors scanned at load time — only `"Audio Module
    /// Class"` entries, in declaration order — the cached, interned
    /// slice (identical on every call; no re-scan or per-call leak
    /// growth).
    ///
    /// **RT contract:** non-RT only — borrowed cached slice.
    #[must_use]
    pub fn descriptors(&self) -> &[PluginDescriptor] {
        &self.descriptors
    }

    /// One bridge factory per scanned descriptor. Param tables come
    /// from the load-time enumeration cache (index-aligned with
    /// `descriptors`) so [`Plugin::params`] works at every lifecycle
    /// point including pre-`init`.
    ///
    /// **RT contract:** non-RT only — allocates the factory vec; no
    /// plugin instantiation or metadata scan happens here.
    #[must_use]
    pub fn factories(&self) -> Vec<Vst3PluginFactory> {
        self.descriptors
            .iter()
            .zip(self.cids.iter())
            .zip(self.params_tables.iter())
            .map(|((&descriptor, &cid), &params)| Vst3PluginFactory {
                descriptor,
                params,
                cid,
                library: Arc::clone(&self.inner),
            })
            .collect()
    }
}

/// The platform module-entry hook, when exported — `InitDll`
/// (Windows), `ModuleEntry` (Linux), `bundleEntry` (macOS). Absent
/// hooks are a defined no-op; a hook declining initialization fails
/// the load.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
fn module_entry(lib: &Library) -> Result<(), Vst3HostError> {
    // SAFETY: `get` resolves the symbol without running module code.
    let Ok(init) = (unsafe { lib.get::<unsafe extern "system" fn() -> bool>(b"InitDll\0") }) else {
        return Ok(());
    };
    // SAFETY: `InitDll` is the module's declared Windows entry hook.
    if unsafe { init() } {
        Ok(())
    } else {
        Err(Vst3HostError::ModuleInit)
    }
}

/// See the Windows `module_entry` — Linux's `ModuleEntry` takes the
/// module image pointer, which the fixture ignores; `null` is the
/// honest minimum (no bundle resources to hand over).
#[cfg(target_os = "linux")]
#[expect(unsafe_code)]
fn module_entry(lib: &Library) -> Result<(), Vst3HostError> {
    // SAFETY: `get` resolves the symbol without running module code.
    let Ok(entry) =
        (unsafe { lib.get::<unsafe extern "system" fn(*mut c_void) -> bool>(b"ModuleEntry\0") })
    else {
        return Ok(());
    };
    // SAFETY: `ModuleEntry` is the module's declared Linux entry hook.
    if unsafe { entry(ptr::null_mut()) } {
        Ok(())
    } else {
        Err(Vst3HostError::ModuleInit)
    }
}

/// See the Windows `module_entry` — macOS's `bundleEntry` takes the
/// bundle reference; `null` for the same reason as Linux's module
/// pointer.
#[cfg(target_os = "macos")]
#[expect(unsafe_code)]
fn module_entry(lib: &Library) -> Result<(), Vst3HostError> {
    // SAFETY: `get` resolves the symbol without running module code.
    let Ok(entry) =
        (unsafe { lib.get::<unsafe extern "system" fn(*mut c_void) -> bool>(b"bundleEntry\0") })
    else {
        return Ok(());
    };
    // SAFETY: `bundleEntry` is the module's declared macOS entry hook.
    if unsafe { entry(ptr::null_mut()) } {
        Ok(())
    } else {
        Err(Vst3HostError::ModuleInit)
    }
}

/// Other platforms export no module hooks in the VST3 spec's
/// per-platform matrix — a defined no-op.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn module_entry(_lib: &Library) -> Result<(), Vst3HostError> {
    Ok(())
}

/// The platform module-exit hook, when exported — stored for
/// `LibraryInner::drop`. Absent → `None`.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
fn module_exit(lib: &Library) -> Option<unsafe extern "system" fn() -> bool> {
    // SAFETY: `get` resolves the symbol without running module code.
    unsafe { lib.get::<unsafe extern "system" fn() -> bool>(b"ExitDll\0") }
        .ok()
        .map(|symbol| *symbol)
}

/// See the Windows `module_exit` — Linux's `ModuleExit` hook.
#[cfg(target_os = "linux")]
#[expect(unsafe_code)]
fn module_exit(lib: &Library) -> Option<unsafe extern "system" fn() -> bool> {
    // SAFETY: `get` resolves the symbol without running module code.
    unsafe { lib.get::<unsafe extern "system" fn() -> bool>(b"ModuleExit\0") }
        .ok()
        .map(|symbol| *symbol)
}

/// See the Windows `module_exit` — macOS's `bundleExit` hook.
#[cfg(target_os = "macos")]
#[expect(unsafe_code)]
fn module_exit(lib: &Library) -> Option<unsafe extern "system" fn() -> bool> {
    // SAFETY: `get` resolves the symbol without running module code.
    unsafe { lib.get::<unsafe extern "system" fn() -> bool>(b"bundleExit\0") }
        .ok()
        .map(|symbol| *symbol)
}

/// Other platforms export no module hooks — always `None`.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn module_exit(_lib: &Library) -> Option<unsafe extern "system" fn() -> bool> {
    None
}

/// The load-time class scan: `countClasses` × the best available
/// `getClassInfo*` (`getClassInfoUnicode` > `getClassInfo2` >
/// `getClassInfo`), filtered to `"Audio Module Class"`.
///
/// A non-`kResultOk` info call is a hard `Err(Com)` — a class that
/// cannot be described is a broken scan, not a skippable detail.
///
/// # Errors
///
/// [`Vst3HostError::Com`] on a failed `getClassInfo*` call.
#[expect(unsafe_code)]
fn scan_classes(
    factory: &ComPtr<IPluginFactory>,
) -> Result<(Vec<PluginDescriptor>, Vec<TUID>), Vst3HostError> {
    // `cast` is a `queryInterface` — `IPluginFactory3` subsumes
    // `IPluginFactory2`/`IPluginFactory` in the bindings' inheritance
    // chain.
    let factory3 = factory.cast::<IPluginFactory3>();
    let factory2 = factory.cast::<IPluginFactory2>();
    // SAFETY: `factory` is a live `IPluginFactory`; `countClasses` is
    // total.
    // `count` is plugin-reported — clamp before it can bound the scan
    // loop (and the per-class intern/instantiate cost).
    let count = unsafe { factory.countClasses() }.clamp(0, MAX_SCAN_CLASSES);
    let mut descriptors = Vec::new();
    let mut cids = Vec::new();
    for index in 0..count {
        let info = if let Some(factory3) = &factory3 {
            let mut info = PClassInfoW {
                cid: [0; 16],
                cardinality: 0,
                category: [0; 32],
                name: [0; 64],
                classFlags: 0,
                subCategories: [0; 128],
                vendor: [0; 64],
                version: [0; 64],
                sdkVersion: [0; 64],
            };
            // SAFETY: `info` is a valid `PClassInfoW` out-param.
            let result = unsafe { factory3.getClassInfoUnicode(index, &raw mut info) };
            if result != kResultOk {
                return Err(Vst3HostError::Com(result));
            }
            ClassInfo::from_w(&info)
        } else if let Some(factory2) = &factory2 {
            let mut info = PClassInfo2 {
                cid: [0; 16],
                cardinality: 0,
                category: [0; 32],
                name: [0; 64],
                classFlags: 0,
                subCategories: [0; 128],
                vendor: [0; 64],
                version: [0; 64],
                sdkVersion: [0; 64],
            };
            // SAFETY: `info` is a valid `PClassInfo2` out-param.
            let result = unsafe { factory2.getClassInfo2(index, &raw mut info) };
            if result != kResultOk {
                return Err(Vst3HostError::Com(result));
            }
            ClassInfo::from_v2(&info)
        } else {
            let mut info = PClassInfo {
                cid: [0; 16],
                cardinality: 0,
                category: [0; 32],
                name: [0; 64],
            };
            // SAFETY: `info` is a valid `PClassInfo` out-param.
            let result = unsafe { factory.getClassInfo(index, &raw mut info) };
            if result != kResultOk {
                return Err(Vst3HostError::Com(result));
            }
            ClassInfo::from_v1(&info)
        };
        if info.category != AUDIO_MODULE_CATEGORY {
            continue;
        }
        descriptors.push(info.descriptor());
        cids.push(info.cid);
    }
    Ok((descriptors, cids))
}

/// `createInstance` + `queryInterface` combined: instantiates `cid`
/// asking directly for interface `I`, wrapped as an owned `ComPtr`.
/// `None` on a non-`kResultOk` result or a null return.
#[expect(unsafe_code)]
fn create_instance<I: Interface>(
    factory: &ComPtr<IPluginFactory>,
    cid: &TUID,
) -> Option<ComPtr<I>> {
    let iid = I::IID;
    let mut obj: *mut c_void = ptr::null_mut();
    // SAFETY: `factory` is a live `IPluginFactory`; `cid`/`iid` point
    // to live 16-byte ids for the call; `obj` is a valid out-param.
    let result = unsafe { factory.createInstance(cid.as_ptr(), iid.as_ptr().cast(), &raw mut obj) };
    if result != kResultOk {
        return None;
    }
    // SAFETY: on `kResultOk` the contract transfers a valid `I`
    // interface pointer (or null → `None`) into `obj`.
    unsafe { ComPtr::from_raw(obj.cast::<I>()) }
}

/// Enumerates the class's params into an interned neutral
/// [`ParamInfo`] table.
///
/// Params discovery needs live objects (the distributed controller
/// owns the table), so a throwaway component + controller pair is
/// instantiated — bounded once per descriptor at [`Vst3Library::load`]
/// and released immediately (`controller.terminate` + `ComPtr` drops).
/// A class reporting its own CID is a single-component plugin: the
/// component *is* the controller, so `cast` resolves the interface on
/// the same object (`createInstance` would fabricate a disconnected
/// clone). A class that cannot instantiate gets an honest empty
/// table; the real failure surfaces at [`Plugin::init`] instead of
/// being fabricated here.
///
/// `min`/`max`/`default` are mapped through `normalizedParamToPlain`
/// so the plain domain is honest for any normalization law; `unit`
/// comes from `ParameterInfo.units`.
#[expect(unsafe_code)]
fn enumerate_params(
    factory: &ComPtr<IPluginFactory>,
    component_cid: &TUID,
) -> &'static [ParamInfo] {
    // SAFETY: `factory` is a live `IPluginFactory`; every returned
    // pointer is an owned COM reference wrapped in `ComPtr`, and all
    // out-params are valid for their calls.
    unsafe {
        let Some(component) = create_instance::<IComponent>(factory, component_cid) else {
            return &[];
        };
        let mut controller_cid: TUID = [0; 16];
        if component.getControllerClassId(&raw mut controller_cid) != kResultOk {
            return &[];
        }
        let Some(controller) = (if controller_cid == *component_cid {
            // Single-component plugin — the component reports its own
            // CID, so `IEditController` lives on the same object.
            component.cast::<IEditController>()
        } else {
            create_instance::<IEditController>(factory, &controller_cid)
        }) else {
            return &[];
        };
        // A minimal host context — the VST3 contract initializes the
        // controller before its param surface is driven. For a
        // single-component plugin this is the shared object's one
        // `initialize` (the component's own is never called here).
        let host = ComWrapper::new(BridgeHostApplication);
        let Some(context) = host.as_com_ref::<FUnknown>() else {
            return &[];
        };
        if controller.initialize(context.as_ptr()) != kResultOk {
            return &[];
        }
        // `count` is plugin-reported — clamp before it sizes the
        // reservation or the `getParameterInfo` scan loop.
        let count = controller.getParameterCount().clamp(0, MAX_SCAN_PARAMS);
        let mut infos = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
        for index in 0..count {
            let mut info = ParameterInfo {
                id: 0,
                title: [0; 128],
                shortTitle: [0; 128],
                units: [0; 128],
                stepCount: 0,
                defaultNormalizedValue: 0.0,
                unitId: 0,
                flags: 0,
            };
            if controller.getParameterInfo(index, &raw mut info) != kResultOk {
                // Deliberate skip: a param the controller cannot
                // describe is unrepresentable — the honest answer is
                // the describable subset, not a fabricated entry or
                // a failed table.
                continue;
            }
            infos.push(ParamInfo {
                id: ParamId(info.id),
                name: intern_str(&wstring_to_string(&info.title)),
                unit: intern_str(&wstring_to_string(&info.units)),
                min_value: controller.normalizedParamToPlain(info.id, 0.0),
                max_value: controller.normalizedParamToPlain(info.id, 1.0),
                default_value: controller
                    .normalizedParamToPlain(info.id, info.defaultNormalizedValue),
                flags: map_param_flags(info.flags, info.stepCount),
            });
        }
        let _ = controller.terminate();
        infos.leak()
    }
}

/// `ParameterInfo.flags` + `stepCount` → neutral [`ParamFlags`].
/// VST3-only refinements (`kIsWrapAround`, `kIsProgramChange`) have no
/// neutral counterpart and are dropped — the v1 defined boundary.
fn map_param_flags(flags: i32, step_count: i32) -> ParamFlags {
    let mut mapped = ParamFlags::NONE;
    if flags & ParameterFlags_::kCanAutomate != 0 {
        mapped = mapped.union(ParamFlags::AUTOMATABLE);
    }
    if flags & ParameterFlags_::kIsReadOnly != 0 {
        mapped = mapped.union(ParamFlags::READONLY);
    }
    if flags & ParameterFlags_::kIsHidden != 0 {
        mapped = mapped.union(ParamFlags::HIDDEN);
    }
    if flags & ParameterFlags_::kIsBypass != 0 {
        mapped = mapped.union(ParamFlags::BYPASS);
    }
    if step_count > 0 || flags & ParameterFlags_::kIsList != 0 {
        mapped = mapped.union(ParamFlags::STEPPED);
    }
    mapped
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Bridge [`PluginFactory`] for one scanned class — carries the cached
/// descriptor and interned param table from load/factory time.
///
/// **RT contract:** non-RT only — registration and instantiation.
pub struct Vst3PluginFactory {
    descriptor: PluginDescriptor,
    params: &'static [ParamInfo],
    cid: TUID,
    /// Shared module handle — the instance created from this factory
    /// can never outlive its mapped code.
    library: Arc<LibraryInner>,
}

impl PluginFactory for Vst3PluginFactory {
    fn descriptor(&self) -> PluginDescriptor {
        self.descriptor
    }

    fn create(&self) -> Box<dyn ErasedPlugin> {
        erase(Vst3Plugin::dormant(
            self.descriptor,
            self.params,
            self.cid,
            Arc::clone(&self.library),
        ))
    }

    // `create_sendable` keeps its `None` default — the plugin holds
    // raw channel pointers (`!Send`), so a `Send` erased plugin
    // cannot be honestly produced.
}

// ---------------------------------------------------------------------------
// Instance
// ---------------------------------------------------------------------------

/// The instance lifecycle — mirrors the fixture's `Initialized →
/// Activated → Processing` chain; `Drop` walks it back down (the
/// ordering is pinned by the component's strict transitions).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    /// `create()` — dormant; no COM objects yet.
    Dormant,
    /// `init` — component+controller instantiated and `initialize`d.
    Initialized,
    /// `prepare` — `setActive(true)` + `setupProcessing` accepted.
    Activated,
    /// `start_processing` — `setProcessing(true)` accepted.
    Processing,
}

/// Bridge [`Plugin`] over the dormant COM objects — `create()` only
/// wires identity; the `createInstance` + `initialize` chain runs in
/// `init` per the lifecycle table in the module docs.
struct Vst3Plugin {
    descriptor: PluginDescriptor,
    params: &'static [ParamInfo],
    /// Instantiation CID for the audio-module class.
    cid: TUID,
    /// `IComponent` — `None` until `init`.
    component: Option<ComPtr<IComponent>>,
    /// `IAudioProcessor` on the same object — `None` until `init`.
    processor: Option<ComPtr<IAudioProcessor>>,
    /// `IEditController` — `None` until `init`. May be a second
    /// interface on the component object (single-component plugin —
    /// see `single_component`).
    controller: Option<ComPtr<IEditController>>,
    /// The component reports its own CID from `getControllerClassId`
    /// — component and controller are one object: `controller` was
    /// `cast`, not `createInstance`d, `initialize` ran once, and
    /// teardown terminates the shared object once.
    single_component: bool,
    /// Bridge-owned host objects — kept alive for the instance's life
    /// (a plugin may retain the `initialize` context / handler).
    host_app: Option<ComPtr<IHostApplication>>,
    handler: Option<ComPtr<IComponentHandler>>,
    /// The open editor view (`IPlugView`) — `None` until `open_gui`.
    /// Must be `removed()` + released before `controller` can
    /// `terminate` (the VST3 view-lifetime rule enforced by
    /// [`Vst3Plugin::close_view`] and `Drop`).
    view: Option<ComPtr<IPlugView>>,
    /// The bridge-owned `IPlugFrame` handed to `setFrame` — kept for
    /// the view's open lifetime and released with it.
    plug_frame: Option<ComPtr<IPlugFrame>>,
    /// The plugin→host GUI-request route the session installed —
    /// shared with `BridgePlugFrame` so `resizeView` reaches the
    /// arbiter (Issue #873). `None` keeps `resizeView` at the
    /// defined `kResultFalse` refusal.
    gui_request_sink: Rc<RefCell<Option<Box<GuiRequestSink>>>>,
    /// The redeemed parent lease + platform the open view last
    /// attached to — `set_gui_parent`/`set_gui_visibility` re-`attach`
    /// after a `removed()` need both.
    gui_parent: Option<(ParentLease, vst3::Steinberg::FIDString)>,
    /// `true` while the open view is `attached` — `false` after a
    /// hide/`removed` that left the view open but detached, so
    /// `close_view` never double-`removed`s.
    gui_attached: bool,
    /// The component↔controller connection pair, when both objects
    /// expose `IConnectionPoint` (distributed plugins only). Held so
    /// `Drop` can `disconnect` before `terminate` — the VST3 contract
    /// wants the link released while both objects are still live.
    component_cp: Option<ComPtr<IConnectionPoint>>,
    controller_cp: Option<ComPtr<IConnectionPoint>>,
    /// Mint counter for [`GuiHandle`] values — instance-local,
    /// monotonically increasing.
    gui_seq: u32,
    phase: Phase,
    /// Negotiated max block + rate from `setupProcessing`.
    max_frames: usize,
    sample_rate: u32,
    /// Negotiated bus channel counts (`prepare`-time `getBusInfo`).
    in_channels: usize,
    out_channels: usize,
    /// Bridge-owned input bounce: `in_channels × max_frames` — VST3
    /// channel pointers are `*mut`, so immutable host input cannot
    /// alias; unconnected channels deliver silence.
    input_scratch: Vec<Vec<f32>>,
    /// Owned output scrap for channels the host did not wire.
    output_scrap: Vec<Vec<f32>>,
    /// Channel-pointer arrays handed to `AudioBusBuffers` —
    /// preallocated at `prepare`, repointed per block.
    input_ptrs: Vec<*mut f32>,
    output_ptrs: Vec<*mut f32>,
    /// One queue wrapper per declared param — the host fill side of
    /// `param_changes`; `process` only clears/pushes inside capacity.
    event_queues: Vec<ComWrapper<BridgeParamQueue>>,
    /// The `IParameterChanges` object handed to `process` — built once
    /// in `prepare`, holds the queue `ComPtr`s.
    param_changes: Option<ComPtr<IParameterChanges>>,
    /// The `IEventList` object handed to `process` as
    /// `ProcessData.inputEvents` — the wrapper the bridge fills per
    /// block from canonical `EventKind::Midi` events (Issue #253).
    event_list: Option<ComWrapper<BridgeEventList>>,
    /// The `IEventList` COM pointer kept for `ProcessData` — built
    /// once in `prepare` beside `event_list`.
    input_events: Option<ComPtr<IEventList>>,
    /// Writable queue set for `ProcessData.outputParameterChanges` —
    /// the plugin→host parameter-reporting direction (Issue #253).
    /// Collected into `pending_reports` after each `process`.
    output_queues: Vec<ComWrapper<BridgeParamQueue>>,
    /// The `IParameterChanges` object handed to `process` as
    /// `outputParameterChanges`.
    output_param_changes: Option<ComPtr<IParameterChanges>>,
    /// The `IEventList` wrapper handed to `process` as
    /// `outputEvents` — the plugin appends channel-voice MIDI here.
    output_event_list: Option<ComWrapper<BridgeEventList>>,
    /// The `IEventList` COM pointer kept for `ProcessData`.
    output_events: Option<ComPtr<IEventList>>,
    /// `IMidiMapping` snapshot captured at `prepare` — the RT path
    /// performs table lookups only, never plugin COM calls (the JUCE
    /// `StoredMidiMapping` precedent).
    midi_map: Option<MidiAssignmentTable>,
    /// The controller's `IMidiMapping` interface — queried at `init`
    /// (non-RT) and kept so `prepare` can snapshot the assignment
    /// table. `None` when the plugin declares no mapping surface.
    midi_mapping: Option<ComPtr<IMidiMapping>>,
    /// Plugin→host reports collected from the output channels after
    /// each `process` — drained through
    /// [`PluginProcessor::drain_output_reports`]. Capacity reserved
    /// at `prepare`; pushes stay inside it (defined drop past it).
    pending_reports: Vec<Event>,
    /// Scratch the `outputEvents` entries are copied into before
    /// translation — ends the `BridgeEventList` borrow before
    /// `pending_reports` is pushed into. Reserved at `prepare`.
    report_event_scratch: Vec<VstEvent>,
    /// Collection-side defined-skip counters for the plugin→host
    /// direction — null channels, out-of-block offsets, unwritable
    /// queues and untranslatable events.
    report_stats: MidiBridgeStats,
    /// Pending plugin→host dirty notifications — shared with the
    /// instance's `BridgeComponentHandler`, whose
    /// `performEdit`/`setDirty`/`restartComponent` enqueue from the
    /// plugin's calls (Issue #891). `drain_dirty_notices` moves the
    /// notices out.
    dirty_notices: Rc<RefCell<DirtyNoticeQueue>>,
    /// Cumulative MIDI translation counters — the observable side of
    /// the defined-skip policy (`crate::midi` module docs).
    midi_stats: MidiBridgeStats,
    /// Shared module ownership — declared last so the `Arc` releases
    /// after every COM pointer above (drop order keeps the module
    /// mapped through the final `release`).
    library: Arc<LibraryInner>,
}

impl Vst3Plugin {
    /// A dormant instance: identity wired, no COM objects yet (the
    /// `createInstance` + `initialize` chain belongs to `init`).
    fn dormant(
        descriptor: PluginDescriptor,
        params: &'static [ParamInfo],
        cid: TUID,
        library: Arc<LibraryInner>,
    ) -> Self {
        Self {
            descriptor,
            params,
            cid,
            component: None,
            processor: None,
            controller: None,
            single_component: false,
            host_app: None,
            handler: None,
            view: None,
            plug_frame: None,
            gui_request_sink: Rc::new(RefCell::new(None)),
            gui_parent: None,
            gui_attached: false,
            component_cp: None,
            controller_cp: None,
            gui_seq: 0,
            phase: Phase::Dormant,
            max_frames: 0,
            sample_rate: 0,
            in_channels: 0,
            out_channels: 0,
            input_scratch: Vec::new(),
            output_scrap: Vec::new(),
            input_ptrs: Vec::new(),
            output_ptrs: Vec::new(),
            event_queues: Vec::new(),
            param_changes: None,
            event_list: None,
            input_events: None,
            output_queues: Vec::new(),
            output_param_changes: None,
            output_event_list: None,
            output_events: None,
            midi_map: None,
            midi_mapping: None,
            pending_reports: Vec::new(),
            report_event_scratch: Vec::new(),
            report_stats: MidiBridgeStats::new(),
            dirty_notices: Rc::new(RefCell::new(DirtyNoticeQueue::new())),
            midi_stats: MidiBridgeStats::new(),
            library,
        }
    }

    /// Channel count of the first audio bus in `dir` — `None` when the
    /// component cannot describe it (`getBusInfo` not honored).
    #[expect(unsafe_code)]
    fn bus_channels(&self, dir: BusDirection) -> Option<usize> {
        let component = self.component.as_ref()?;
        // SAFETY: `component` is a live `IComponent`; `info` is a
        // valid `BusInfo` out-param.
        unsafe {
            if component.getBusCount(sdk_enum_i32(MediaTypes_::kAudio), dir) < 1 {
                return None;
            }
            let mut info = BusInfo {
                mediaType: 0,
                direction: 0,
                channelCount: 0,
                name: [0; 128],
                busType: 0,
                flags: 0,
            };
            if component.getBusInfo(sdk_enum_i32(MediaTypes_::kAudio), dir, 0, &raw mut info)
                != kResultOk
            {
                return None;
            }
            // `channelCount` is plugin-reported — clamp before it
            // multiplies the per-channel scratch at `prepare`.
            usize::try_from(info.channelCount)
                .ok()
                .filter(|&n| n > 0)
                .map(|n| n.min(MAX_SCAN_CHANNELS))
        }
    }

    /// Tears down the open editor view, if any: `removed()` then
    /// release the view and frame. The VST3 lifetime rule — a view
    /// must be `removed`ed and released before its controller can
    /// `terminate` — is enforced here so `Drop` can run it before
    /// the lifecycle descent. Idempotent: a second call is a no-op.
    #[expect(unsafe_code)]
    fn close_view(&mut self) {
        if let Some(view) = self.view.take() {
            // SAFETY: `view` is a live `IPlugView` created by
            // `open_gui`; `removed` is the required detach before
            // release and runs on the owning UI thread. `setFrame(0)`
            // clears the plugin's frame pointer defensively (some
            // plugins hold it without addref) before our release.
            // `gui_attached` gates the call — a hidden view was
            // already `removed()` by `set_gui_visibility`, and a
            // second `removed` is not part of the VST3 contract.
            if self.gui_attached {
                let result = unsafe { view.removed() };
                debug_assert_eq!(result, kResultOk, "view.removed() declined");
            }
            let _ = unsafe { view.setFrame(ptr::null_mut()) };
            drop(view);
        }
        self.gui_attached = false;
        self.gui_parent = None;
        // The request route dies with the view — a stale `resizeView`
        // must meet the defined refusal, not an arbiter for a dead
        // session. A shared `RefCell` borrow from an in-flight
        // request is released before `close_view` can run, so a plain
        // `borrow_mut` cannot panic here.
        *self.gui_request_sink.borrow_mut() = None;
        drop(self.plug_frame.take());
    }

    /// The `GuiFocusTarget::HostWindow` arm (Windows): keyboard focus
    /// moves back to the container HWND the session attached the view
    /// under — the stored `gui_parent` payload — and the plugin is
    /// told of the loss through `onFocus(false)`, which is what the
    /// fixture's `f:0` token records. `GetFocus()==container` is the
    /// oracle that proves the handoff actually happened; the
    /// `SetFocus` return value cannot witness it (a window that held
    /// no prior focus returns null on success too). Same UI-thread
    /// contract as `open_gui` — `SetFocus` only reaches windows the
    /// calling thread owns.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no parent lease is stored;
    /// [`HostError::PluginFailed`] when focus did not move to the
    /// container or the `onFocus` notification declined.
    #[cfg(target_os = "windows")]
    #[expect(unsafe_code)]
    fn focus_host_window(&self, view: &ComPtr<IPlugView>) -> Result<(), HostError> {
        let Some((parent, _platform)) = self.gui_parent else {
            return Err(HostError::Unsupported);
        };
        let hwnd = parent.payload() as *mut c_void;
        if !win32::set_focus(hwnd) {
            return Err(HostError::PluginFailed);
        }
        // SAFETY: `view` is a live `IPlugView` on the caller's UI
        // thread — the `open_gui` contract.
        if unsafe { view.onFocus(TBool::from(false)) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        Ok(())
    }

    /// Off-Windows the host-window focus handoff has no implemented
    /// OS surface — the defined refusal.
    #[cfg(not(target_os = "windows"))]
    fn focus_host_window(&self, _view: &ComPtr<IPlugView>) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }

    /// Builds the per-`prepare` channel objects `process` drives:
    /// read-only input param queues + `IParameterChanges`, their
    /// writable output mirrors for `outputParameterChanges`, the
    /// input/output `IEventList` shells, the `IMidiMapping` snapshot,
    /// and the report-collection buffers. All storage is preallocated
    /// — `process` only clears/fills inside capacity (the module
    /// docs' no-allocation RT contract). Non-RT: allocation is legal
    /// here.
    fn prepare_channels(&mut self) {
        // One preallocated queue per declared param plus the
        // `IParameterChanges` shell — `process` only fills inside
        // capacity (the module docs' no-allocation RT contract).
        self.event_queues = self
            .params
            .iter()
            .map(|info| {
                ComWrapper::new(BridgeParamQueue {
                    id: info.id.0,
                    writable: false,
                    points: RefCell::new(Vec::with_capacity(MAX_EVENTS_PER_BLOCK)),
                })
            })
            .collect();
        let queues: Vec<(u32, ComPtr<IParamValueQueue>)> = self
            .event_queues
            .iter()
            .filter_map(|queue| {
                queue
                    .to_com_ptr::<IParamValueQueue>()
                    .map(|ptr| (queue.id, ptr))
            })
            .collect();
        self.param_changes = ComWrapper::new(BridgeParameterChanges {
            queues,
            writable: false,
        })
        .to_com_ptr::<IParameterChanges>();
        // The writable mirror for `ProcessData.outputParameterChanges`
        // — same declared-param queue set, plugin-filled. Built once
        // here; `process` clears/collects inside capacity.
        self.output_queues = self
            .params
            .iter()
            .map(|info| {
                ComWrapper::new(BridgeParamQueue {
                    id: info.id.0,
                    writable: true,
                    points: RefCell::new(Vec::with_capacity(MAX_EVENTS_PER_BLOCK)),
                })
            })
            .collect();
        let output_queues: Vec<(u32, ComPtr<IParamValueQueue>)> = self
            .output_queues
            .iter()
            .filter_map(|queue| {
                queue
                    .to_com_ptr::<IParamValueQueue>()
                    .map(|ptr| (queue.id, ptr))
            })
            .collect();
        self.output_param_changes = ComWrapper::new(BridgeParameterChanges {
            queues: output_queues,
            writable: true,
        })
        .to_com_ptr::<IParameterChanges>();
        // The `IEventList` shell for `ProcessData.inputEvents` —
        // capacity reserved once here; `process` only clears/pushes
        // inside it (the same no-allocation RT contract as the
        // param queues).
        let event_list = ComWrapper::new(BridgeEventList::with_capacity());
        self.input_events = event_list.to_com_ptr::<IEventList>();
        self.event_list = Some(event_list);
        // The writable mirror for `ProcessData.outputEvents` — the
        // plugin appends channel-voice MIDI here; `process` collects
        // it into `pending_reports` after the call.
        let output_event_list = ComWrapper::new(BridgeEventList::with_capacity());
        self.output_events = output_event_list.to_com_ptr::<IEventList>();
        self.output_event_list = Some(output_event_list);
        // Snapshot the `IMidiMapping` table off the RT path — the
        // plugin may change assignments between `prepare` calls, so
        // the capture repeats here rather than at `init`.
        self.midi_map = self.midi_mapping.as_ref().map(MidiAssignmentTable::capture);
        // Report collection buffers — reserved once; `process` pushes
        // inside capacity (defined drop past it).
        self.pending_reports.clear();
        self.pending_reports.reserve(MAX_EVENTS_PER_BLOCK);
        self.report_event_scratch.clear();
        self.report_event_scratch.reserve(block::MAX_BLOCK_EVENTS);
    }

    /// Copies `frames` from every wired host input channel into
    /// `input_scratch` (unwired channels deliver silence — inputs
    /// cannot alias the plugin's output buffers) and repoints the
    /// channel-pointer arrays: inputs at scratch, outputs at the
    /// host's buffers where wired and `output_scrap` where not.
    ///
    /// **RT contract:** RT-suitable — bounded copies into pre-sized
    /// scratch, no allocation.
    fn bounce_audio<I, O>(&mut self, inputs: &[I], outputs: &mut [O], frames: usize)
    where
        I: BufferView,
        O: BufferViewMut,
    {
        // Input bounce — copy `frames` from every wired host channel
        // into scratch; a channel the host did not wire delivers
        // silence (see the module docs for why inputs cannot alias).
        for (index, channel) in self.input_scratch.iter_mut().enumerate() {
            let buffer = &mut channel[..frames];
            match inputs.first().and_then(|bus| bus.channel(index)) {
                Some(source) if source.len() >= frames => {
                    buffer.copy_from_slice(&source[..frames]);
                }
                _ => buffer.fill(0.0),
            }
        }
        // Repoint the channel-pointer arrays — preallocated `Vec`s,
        // `iter_mut` keeps their length fixed (no realloc).
        for (ptr, channel) in self
            .input_ptrs
            .iter_mut()
            .zip(self.input_scratch.iter_mut())
        {
            *ptr = channel.as_mut_ptr();
        }
        for (index, ptr) in self.output_ptrs.iter_mut().enumerate() {
            *ptr = match outputs.get_mut(0).and_then(|bus| bus.channel_mut(index)) {
                Some(target) if target.len() >= frames => target.as_mut_ptr(),
                _ => self
                    .output_scrap
                    .get_mut(index)
                    .map_or(ptr::null_mut(), Vec::as_mut_ptr),
            };
        }
    }

    /// Collects the plugin→host reports the just-finished `process`
    /// wrote into `outputParameterChanges`/`outputEvents` into
    /// `pending_reports` — param-queue points become
    /// [`EventKind::ParamValue`] reports, output event-list entries
    /// go back through [`midi::incoming_event`] into
    /// [`EventKind::Midi`] reports. Runs inside `process` (still the
    /// RT path): bounded iteration over preallocated storage, pushes
    /// inside reserved capacity — a full buffer or an untranslatable
    /// entry is a defined skip counted in `report_stats`, never an
    /// allocation. Reports accumulate across blocks until drained —
    /// the bounded `pending_reports` capacity is the back-pressure
    /// bound the neutral contract declares.
    fn collect_output_reports(&mut self, frames: usize) {
        self.report_event_scratch.clear();
        let frame_bound = u32::try_from(frames).unwrap_or(u32::MAX);
        // Parameter reports — every writable queue's points become
        // `ParamValue` events at their sample offsets.
        for queue in &self.output_queues {
            let Ok(points) = queue.points.try_borrow() else {
                self.report_stats.skipped += 1;
                continue;
            };
            for &(offset, value) in points.iter() {
                let frame = u32::try_from(offset).unwrap_or(u32::MAX);
                // `ParamValue::new` (plugin-core — the vst3 `ParamValue`
                // alias above is the `f64` typedef) clamps finite input
                // into `[0,1]` and rejects non-finite — a plugin
                // reporting NaN/inf or an out-of-block offset is a
                // defined skip, not a fabricated value.
                let Some(value) = ruviform_plugin_core::ParamValue::new(value).ok() else {
                    self.report_stats.skipped += 1;
                    continue;
                };
                if frame >= frame_bound {
                    self.report_stats.skipped += 1;
                    continue;
                }
                push_report(
                    &mut self.pending_reports,
                    &mut self.report_stats,
                    Event::new(
                        frame,
                        EventKind::ParamValue(ParamEvent {
                            param_id: ParamId(queue.id),
                            value,
                        }),
                    ),
                );
            }
        }
        // MIDI reports — copy the plugin's `outputEvents` entries into
        // scratch first so the `BridgeEventList` borrow ends before
        // the report pushes.
        if let Some(list) = self.output_event_list.as_ref()
            && !list.copy_events_into(&mut self.report_event_scratch)
        {
            self.report_stats.skipped += 1;
        }
        for raw in self.report_event_scratch.iter().take(MAX_EVENTS_PER_BLOCK) {
            match midi::incoming_event(raw, frames) {
                Some(event) => {
                    push_report(&mut self.pending_reports, &mut self.report_stats, event);
                }
                None => self.report_stats.skipped += 1,
            }
        }
        // Canonical consumers expect frame-ordered reports; the sort
        // is unstable (no allocation) over the reserved buffer.
        self.pending_reports
            .sort_unstable_by_key(|event| event.frame_offset());
        // Next block starts clean — the output channels are per-call
        // objects; clearing here keeps a plugin that forgets to is
        // still correct.
        for queue in &self.output_queues {
            if let Ok(mut points) = queue.points.try_borrow_mut() {
                points.clear();
            }
        }
        if let Some(list) = self.output_event_list.as_ref() {
            list.clear();
        }
    }
}

impl Drop for Vst3Plugin {
    /// Symmetric teardown, best-effort — the neutral surface has no
    /// `Deactivate`/`Destroy` hook, so they run here while `library`
    /// (the last field) still keeps the module mapped:
    /// `setProcessing(false)` → `setActive(false)` → `terminate` on
    /// both objects. The fixture rejects `setActive(false)` while
    /// `Processing` and `terminate` while `Activated`, so the descent
    /// order is pinned.
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // The VST3 view-lifetime rule pins this first: a view must be
        // `removed`ed + released before its controller can
        // `terminate` — `close_gui` may already have done it, so the
        // helper is a defined no-op then.
        self.close_view();
        // SAFETY: each `Option` COM pointer is a live object owned by
        // `self`; calls are skipped (not made) when absent.
        unsafe {
            // Each call's tresult is asserted in debug builds — the
            // fixture pins every legal descent transition to
            // `kResultOk`, so a declined step (a wrong order, a
            // non-compliant plugin) surfaces in tests while release
            // stays best-effort.
            if self.phase == Phase::Processing
                && let Some(processor) = &self.processor
            {
                let result = processor.setProcessing(0);
                debug_assert_eq!(result, kResultOk, "setProcessing(false) declined");
            }
            if self.phase >= Phase::Activated
                && let Some(component) = &self.component
            {
                let result = component.setActive(0);
                debug_assert_eq!(result, kResultOk, "setActive(false) declined");
            }
            if self.phase >= Phase::Initialized {
                // Release the component↔controller link while both
                // objects are still live — the connection contract's
                // own teardown order (disconnect before terminate).
                if let (Some(c_cp), Some(e_cp)) =
                    (self.component_cp.take(), self.controller_cp.take())
                {
                    let _ = e_cp.disconnect(c_cp.as_ptr());
                    let _ = c_cp.disconnect(e_cp.as_ptr());
                }
                // A distributed controller is a separate object with
                // its own `terminate`; a single-component plugin
                // shares the component's — terminating the shared
                // object twice would decline the second call.
                if !self.single_component
                    && let Some(controller) = &self.controller
                {
                    let result = controller.terminate();
                    debug_assert_eq!(result, kResultOk, "controller.terminate declined");
                }
                if let Some(component) = &self.component {
                    let result = component.terminate();
                    debug_assert_eq!(result, kResultOk, "component.terminate declined");
                }
            }
        }
    }
}

/// The smallest honest frame count for the block — the declared block
/// size (`EventList`'s `block_frames`) clamped to the shortest wired
/// input/output channel slice (defensive; views promise equal
/// lengths). Callers negotiate `max_frames` at `prepare`.
fn block_frames<I: BufferView, O: BufferViewMut>(
    inputs: &[I],
    outputs: &[O],
    declared: usize,
) -> usize {
    let mut frames = declared;
    for bus in inputs {
        for index in 0..bus.channels() {
            if let Some(channel) = bus.channel(index) {
                frames = frames.min(channel.len());
            }
        }
    }
    for bus in outputs {
        for index in 0..bus.channels() {
            if let Some(channel) = bus.channel(index) {
                frames = frames.min(channel.len());
            }
        }
    }
    frames
}

/// Distributes the block's parameter-bound events onto the
/// preallocated queues — `clear()` every queue, then push
/// `(sample_offset, normalized_value)` points inside capacity.
/// Sources: `EventKind::ParamValue` events directly, and
/// `EventKind::Midi` events consumed by `IMidiMapping` routing
/// (`midi_map` + [`midi::routed_assignment`] — the same predicate
/// [`midi::fill_event_list`] uses for suppression, so a routed event
/// can never be delivered twice or lost between the two paths).
/// Foreign param ids (no queue exists), out-of-block offsets and
/// events past `MAX_EVENTS_PER_BLOCK` are dropped by definition —
/// never an allocation or a wrong-frame clamp. Returns the per-call
/// stats: `param_routed` counts MIDI events the mapping consumed;
/// `translated`/`skipped` count queue deliveries and drops.
fn fill_param_queues(
    queues: &[ComWrapper<BridgeParamQueue>],
    events: EventList<'_>,
    frames: usize,
    midi_map: Option<&MidiAssignmentTable>,
) -> MidiBridgeStats {
    let mut stats = MidiBridgeStats::new();
    for queue in queues {
        if let Ok(mut points) = queue.points.try_borrow_mut() {
            points.clear();
        }
    }
    let frame_bound = u32::try_from(frames).unwrap_or(u32::MAX);
    for event in events.iter().take(MAX_EVENTS_PER_BLOCK) {
        let (param_id, value) = match event.kind() {
            EventKind::ParamValue(param) => (param.param_id.0, param.value.get()),
            EventKind::Midi(midi_event) => match midi::routed_assignment(&midi_event, midi_map) {
                Some((id, value)) => {
                    stats.param_routed += 1;
                    (id, value)
                }
                None => continue,
            },
            _ => continue,
        };
        if event.frame_offset() >= frame_bound {
            stats.skipped += 1;
            continue;
        }
        let offset = i32::try_from(event.frame_offset()).unwrap_or(i32::MAX);
        let Some(queue) = queues.iter().find(|queue| queue.id == param_id) else {
            stats.skipped += 1;
            continue;
        };
        let Ok(mut points) = queue.points.try_borrow_mut() else {
            stats.skipped += 1;
            continue;
        };
        if points.len() < MAX_EVENTS_PER_BLOCK {
            points.push((offset, value));
            stats.translated += 1;
        } else {
            stats.skipped += 1;
        }
    }
    stats
}

/// Maps the optional neutral transport snapshot onto a VST3
/// `ProcessContext`: `Playing` → `kPlaying`; `position` →
/// `projectTimeSamples` in the negotiated sample base; the musical
/// snapshot → PPQ position/bar, tempo and time-signature fields with
/// their `*Valid` flags. `None` transport → `None` context (the
/// fixture tolerates a null context — the honest free-running
/// boundary).
///
/// `continousTimeSamples` stays zero and `kContTimeValid` unset:
/// `position()` is the transport playhead, not engine-continuous
/// time, so writing the field without its validity flag would be a
/// dead write spec-conforming readers must ignore anyway.
fn map_process_context(
    transport: Option<TransportInfo>,
    sample_rate: u32,
) -> Option<VstProcessContext> {
    let info = transport?;
    let mut context = VstProcessContext {
        state: 0,
        sampleRate: f64::from(sample_rate),
        projectTimeSamples: 0,
        systemTime: 0,
        continousTimeSamples: 0,
        projectTimeMusic: 0.0,
        barPositionMusic: 0.0,
        cycleStartMusic: 0.0,
        cycleEndMusic: 0.0,
        tempo: 0.0,
        timeSigNumerator: 0,
        timeSigDenominator: 0,
        chord: Chord {
            keyNote: 0,
            rootNote: 0,
            chordMask: 0,
        },
        smpteOffsetSubframes: 0,
        frameRate: FrameRate {
            framesPerSecond: 0,
            flags: 0,
        },
        samplesToNextClock: 0,
    };
    if info.status() == TransportStatus::Playing {
        context.state |= ProcessContext_::StatesAndFlags_::kPlaying as u32;
    }
    if let Ok(base) = TimeBase::sample_rate(sample_rate)
        && let Ok(position) = info.position().convert(base, Rounding::NearestEven)
    {
        context.projectTimeSamples = position.value();
    }
    if let Some(musical) = info.musical() {
        let ppqn = f64::from(musical.ppqn);
        // i64 → f64 is the defined tick → quarter-note mapping —
        // precision loss for positions beyond 2^53 ticks is accepted.
        #[expect(clippy::cast_precision_loss)]
        {
            context.projectTimeMusic = musical.position_ticks as f64 / ppqn;
            context.barPositionMusic = musical.bar_start_tick as f64 / ppqn;
        }
        context.tempo = f64::from(musical.bpm_milli) / 1_000.0;
        context.state |= (ProcessContext_::StatesAndFlags_::kProjectTimeMusicValid
            | ProcessContext_::StatesAndFlags_::kTempoValid) as u32;
        if musical.sig_num != 0 && musical.sig_den != 0 {
            context.timeSigNumerator = i32::from(musical.sig_num);
            context.timeSigDenominator = i32::from(musical.sig_den);
            context.state |= (ProcessContext_::StatesAndFlags_::kTimeSigValid
                | ProcessContext_::StatesAndFlags_::kBarPositionValid)
                as u32;
        }
    }
    Some(context)
}

impl PluginProcessor for Vst3Plugin {
    /// `prepare` → `setActive(true)` → `setupProcessing` — in that
    /// order, pinned by the component's `Activated` precondition —
    /// then sizes every buffer the RT call needs: input bounce
    /// scratch (`in_channels × max_frames`), output scrap, the
    /// channel-pointer arrays and the param-change objects.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`PrepareError::UnsupportedConfig`] when `max_frames` exceeds
    /// the wire protocol's [`WIRE_MAX_FRAMES`] ceiling — the
    /// negotiated bound sizes the scratch, so it is rejected before
    /// it can drive an oversized allocation;
    /// [`PrepareError::ActivationFailed`] when the instance is not
    /// `Initialized` or `setActive`/`setupProcessing` declines.
    #[expect(unsafe_code)]
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        if self.phase != Phase::Initialized {
            return Err(PrepareError::ActivationFailed);
        }
        let (Some(component), Some(processor)) = (self.component.as_ref(), self.processor.as_ref())
        else {
            return Err(PrepareError::ActivationFailed);
        };
        // The negotiated bound sizes `input_scratch`/`output_scrap`
        // below — reject above the wire ceiling before it can drive
        // an oversized allocation (same bound the CLAP bridge pins).
        if config.max_frames() > WIRE_MAX_FRAMES {
            return Err(PrepareError::UnsupportedConfig);
        }
        // `max_frames` ≤ `WIRE_MAX_FRAMES` (8_192) — the `i32`
        // conversion cannot fail.
        let max_samples = i32::try_from(config.max_frames()).unwrap_or(i32::MAX);
        // SAFETY: `component`/`processor` are live COM objects;
        // `setup` is a valid `ProcessSetup` in-param for the call.
        unsafe {
            if component.setActive(1) != kResultOk {
                return Err(PrepareError::ActivationFailed);
            }
            let mut setup = ProcessSetup {
                processMode: sdk_enum_i32(ProcessModes_::kRealtime),
                symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
                maxSamplesPerBlock: max_samples,
                sampleRate: f64::from(config.sample_rate().get()),
            };
            if processor.setupProcessing(&raw mut setup) != kResultOk {
                // Roll the activation back so the component is left in
                // a legal state — teardown requires `Initialized`.
                let _ = component.setActive(0);
                return Err(PrepareError::ActivationFailed);
            }
        }
        self.phase = Phase::Activated;
        self.max_frames = usize::try_from(config.max_frames()).unwrap_or(usize::MAX);
        self.sample_rate = config.sample_rate().get();
        let in_channels = self
            .bus_channels(sdk_enum_i32(BusDirections_::kInput))
            .unwrap_or(CHANNELS_PER_BUS);
        let out_channels = self
            .bus_channels(sdk_enum_i32(BusDirections_::kOutput))
            .unwrap_or(CHANNELS_PER_BUS);
        self.in_channels = in_channels;
        self.out_channels = out_channels;
        self.input_scratch = (0..in_channels)
            .map(|_| vec![0.0; self.max_frames])
            .collect();
        self.output_scrap = (0..out_channels)
            .map(|_| vec![0.0; self.max_frames])
            .collect();
        self.input_ptrs = vec![ptr::null_mut(); in_channels];
        self.output_ptrs = vec![ptr::null_mut(); out_channels];
        self.prepare_channels();
        Ok(())
    }

    /// `process` → `IAudioProcessor::process(ProcessData)` — see the
    /// module docs for the input bounce, direct output wiring, event
    /// queues and `processContext` mapping. Exactly one input bus and
    /// one output bus, `kSample32`, `kRealtime`.
    ///
    /// **RT contract:** RT-suitable — bounded copies into pre-sized
    /// scratch, no allocation, no blocking.
    #[expect(unsafe_code)]
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let error = || ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE);
        if self.phase != Phase::Processing {
            // `process` outside Processing — a defined rejection, not
            // a fabricated transition.
            return error();
        }
        let frames = block_frames(
            inputs,
            outputs,
            usize::try_from(events.block_frames()).unwrap_or(usize::MAX),
        );
        if frames > self.max_frames {
            // A block beyond the negotiated maximum cannot be honestly
            // split in v1 — defined rejection, like the plugin-side
            // adapter's own oversized-block path.
            return error();
        }

        self.bounce_audio(inputs, outputs, frames);

        // Distribute parameter-bound events into the preallocated
        // queues — `ParamValue` events directly plus `Midi` events the
        // `IMidiMapping` snapshot routes to parameters.
        let param_stats =
            fill_param_queues(&self.event_queues, events, frames, self.midi_map.as_ref());
        self.midi_stats.translated += param_stats.translated;
        self.midi_stats.skipped += param_stats.skipped;
        self.midi_stats.param_routed += param_stats.param_routed;
        // Translate canonical MIDI events into the preallocated
        // `IEventList` — `ParamValue`/unknown kinds are not MIDI and
        // are not counted; mapping-routed events are suppressed here
        // (delivered via the queues above); defined skips accumulate
        // in `midi_stats`.
        if let Some(event_list) = self.event_list.as_ref() {
            let stats = midi::fill_event_list(
                event_list,
                events,
                transport.and_then(TransportInfo::musical),
                self.sample_rate,
                self.midi_map.as_ref(),
            );
            self.midi_stats.translated += stats.translated;
            self.midi_stats.skipped += stats.skipped;
        }

        let mut process_context = map_process_context(transport, self.sample_rate);
        let mut in_bus = AudioBusBuffers {
            numChannels: i32::try_from(self.input_ptrs.len()).unwrap_or(i32::MAX),
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: self.input_ptrs.as_mut_ptr(),
            },
        };
        let mut out_bus = AudioBusBuffers {
            numChannels: i32::try_from(self.output_ptrs.len()).unwrap_or(i32::MAX),
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: self.output_ptrs.as_mut_ptr(),
            },
        };
        let mut data = ProcessData {
            processMode: sdk_enum_i32(ProcessModes_::kRealtime),
            symbolicSampleSize: sdk_enum_i32(SymbolicSampleSizes_::kSample32),
            numSamples: i32::try_from(frames).unwrap_or(i32::MAX),
            numInputs: 1,
            numOutputs: 1,
            inputs: &raw mut in_bus,
            outputs: &raw mut out_bus,
            inputParameterChanges: self
                .param_changes
                .as_ref()
                .map_or(ptr::null_mut(), ComPtr::as_ptr),
            outputParameterChanges: self
                .output_param_changes
                .as_ref()
                .map_or(ptr::null_mut(), ComPtr::as_ptr),
            inputEvents: self
                .input_events
                .as_ref()
                .map_or(ptr::null_mut(), ComPtr::as_ptr),
            outputEvents: self
                .output_events
                .as_ref()
                .map_or(ptr::null_mut(), ComPtr::as_ptr),
            processContext: process_context
                .as_mut()
                .map_or(ptr::null_mut(), ptr::from_mut),
        };
        // SAFETY: every pointer in `data` is valid for the call — the
        // channel-pointer arrays and scratch live in `self`, the bus
        // structs, context, changes and event lists are owned by
        // `self` (or locals) alive across the call; `numChannels`/
        // `numSamples` match their arrays. The `processor` borrow is
        // taken last so the `&mut self` report collection below can
        // run once the call returns.
        let Some(processor) = self.processor.as_ref() else {
            return error();
        };
        let result = unsafe { processor.process(&raw mut data) };
        if result == kResultOk {
            // Plugin→host reports: collect what the plugin wrote into
            // `outputParameterChanges`/`outputEvents` while the
            // objects are still call-scoped — a declined `process`
            // leaves the channels in an undefined state, so nothing
            // is collected for it.
            self.collect_output_reports(frames);
            // SAFETY: live COM object; `getTailSamples` reports the
            // tail the just-completed `process` recorded. Re-borrowed
            // — the earlier `processor` loan ended with the `process`
            // call so the report collection could take `&mut self`.
            let tail = unsafe {
                self.processor
                    .as_ref()
                    .map_or(0, |processor| processor.getTailSamples())
            };
            ProcessResult::new(ProcessStatus::Continue, tail)
        } else {
            error()
        }
    }

    /// Moves the reports collected from the plugin's
    /// `outputParameterChanges`/`outputEvents` into `out` — the
    /// consumer-facing half of the plugin→host reporting path.
    ///
    /// **RT contract:** non-RT only — the `out` append may allocate.
    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        out.append(&mut self.pending_reports);
    }
}

/// Appends one collected report inside reserved capacity — a full
/// buffer is the defined drop counted in `stats.skipped`.
///
/// **RT contract:** RT-suitable — bounded push, no allocation.
fn push_report(reports: &mut Vec<Event>, stats: &mut MidiBridgeStats, event: Event) {
    if reports.len() < reports.capacity() {
        reports.push(event);
        stats.translated += 1;
    } else {
        stats.skipped += 1;
    }
}

// ---------------------------------------------------------------------------
// Standard state verbs (non-RT)
// ---------------------------------------------------------------------------

/// Maps a state-verb `tresult` onto the neutral error surface:
/// `kResultOk` succeeds, `kNotImplemented` is the honest capability
/// decline (`HostError::Unsupported` — the same answer the existing
/// L1 e2e pins for the stateless fixture), every other failure is a
/// plugin failure.
fn state_result(result: tresult) -> Result<(), HostError> {
    if result == kResultOk {
        Ok(())
    } else if result == kNotImplemented {
        Err(HostError::Unsupported)
    } else {
        Err(HostError::PluginFailed)
    }
}

/// Smallest blob [`frame_state`] can emit — the two `u32 LE`
/// length headers with both payloads empty. Anything shorter cannot
/// be this adapter's serialization and is foreign input.
const FRAME_HEADER_BYTES: usize = 8;

/// Frames the component/controller payloads into the neutral blob —
/// `[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl]`. Each payload is
/// capped by [`WIRE_MAX_STATE_BYTES`] inside `BridgeStream`, so the
/// `u32` lengths cannot truncate.
fn frame_state(comp: &[u8], ctrl: &[u8]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(8 + comp.len() + ctrl.len());
    blob.extend_from_slice(&u32::try_from(comp.len()).unwrap_or(u32::MAX).to_le_bytes());
    blob.extend_from_slice(comp);
    blob.extend_from_slice(&u32::try_from(ctrl.len()).unwrap_or(u32::MAX).to_le_bytes());
    blob.extend_from_slice(ctrl);
    blob
}

/// Parses the framed blob back into `(component, controller)`
/// payloads — `None` on any malformed shape (short header, length
/// mismatch, trailing bytes). Callers must only feed it blobs of at
/// least [`FRAME_HEADER_BYTES`]; shorter input is foreign state, not
/// a truncated frame.
fn unframe_state(blob: &[u8]) -> Option<(&[u8], &[u8])> {
    let comp_len = u32::from_le_bytes(blob.get(..4)?.try_into().ok()?) as usize;
    let comp_end = 4usize.checked_add(comp_len)?;
    let comp = blob.get(4..comp_end)?;
    let ctrl_len_end = comp_end.checked_add(4)?;
    let ctrl_len = u32::from_le_bytes(blob.get(comp_end..ctrl_len_end)?.try_into().ok()?) as usize;
    let ctrl_end = ctrl_len_end.checked_add(ctrl_len)?;
    let ctrl = blob.get(ctrl_len_end..ctrl_end)?;
    (ctrl_end == blob.len()).then_some((comp, ctrl))
}

/// Runs the VST3 save sequence against the live COM objects:
/// `component.getState` into a fresh `BridgeStream`, then — only for
/// distributed plugins (`single_component == false`) —
/// `controller.getState` into a second stream. Returns the two raw
/// payloads; the caller frames them.
#[expect(unsafe_code)]
fn save_component_state(
    component: &ComPtr<IComponent>,
    controller: Option<&ComPtr<IEditController>>,
    single_component: bool,
) -> Result<(Vec<u8>, Vec<u8>), HostError> {
    let comp_stream = ComWrapper::new(BridgeStream::new());
    let Some(comp_ptr) = comp_stream.to_com_ptr::<IBStream>() else {
        return Err(HostError::PluginFailed);
    };
    // SAFETY: `component` is a live `IComponent` owned by the caller;
    // `comp_ptr` is a valid `IBStream` whose `ComWrapper` outlives
    // the call.
    state_result(unsafe { component.getState(comp_ptr.as_ptr()) })?;
    let comp_bytes = comp_stream.take_bytes();

    if single_component {
        // The component *is* the controller — a second `getState`
        // would re-serialize the same object.
        return Ok((comp_bytes, Vec::new()));
    }
    let Some(controller) = controller else {
        return Err(HostError::PluginFailed);
    };
    let ctrl_stream = ComWrapper::new(BridgeStream::new());
    let Some(ctrl_ptr) = ctrl_stream.to_com_ptr::<IBStream>() else {
        return Err(HostError::PluginFailed);
    };
    // SAFETY: same contract as the component call above.
    state_result(unsafe { controller.getState(ctrl_ptr.as_ptr()) })?;
    Ok((comp_bytes, ctrl_stream.take_bytes()))
}

/// Runs the VST3 restore sequence in the spec's order:
/// `component.setState` → `component.getState` (re-serializes the
/// normalized component state the controller consumes) →
/// `controller.setComponentState` → `controller.setState`.
/// Single-component plugins stop after the component pair — the
/// controller calls have no separate object to drive.
///
/// Blobs of at least [`FRAME_HEADER_BYTES`] must parse as this
/// adapter's framing — a corrupt frame is `PluginFailed`. Shorter
/// blobs cannot be our serialization, so they are foreign state
/// delegated to the plugin as the raw component payload: the plugin
/// owns its state format and answers honestly (the stateless
/// fixture's `kNotImplemented` still maps to `Unsupported`, the
/// decline the L1 e2e pins).
#[expect(unsafe_code)]
fn restore_component_state(
    component: &ComPtr<IComponent>,
    controller: Option<&ComPtr<IEditController>>,
    single_component: bool,
    blob: &[u8],
) -> Result<(), HostError> {
    let (comp_bytes, ctrl_bytes) = if blob.len() < FRAME_HEADER_BYTES {
        (blob, &[][..])
    } else {
        let Some((comp, ctrl)) = unframe_state(blob) else {
            return Err(HostError::PluginFailed);
        };
        (comp, ctrl)
    };
    let comp_in = ComWrapper::new(BridgeStream::from_bytes(comp_bytes.to_vec()));
    let Some(comp_in_ptr) = comp_in.to_com_ptr::<IBStream>() else {
        return Err(HostError::PluginFailed);
    };
    // SAFETY: `component` is a live `IComponent`; `comp_in_ptr` is a
    // valid `IBStream` positioned at 0 holding `comp_bytes`.
    state_result(unsafe { component.setState(comp_in_ptr.as_ptr()) })?;

    let norm = ComWrapper::new(BridgeStream::new());
    let Some(norm_ptr) = norm.to_com_ptr::<IBStream>() else {
        return Err(HostError::PluginFailed);
    };
    // SAFETY: same contract — `norm_ptr` is the `getState` target.
    state_result(unsafe { component.getState(norm_ptr.as_ptr()) })?;
    if single_component {
        return Ok(());
    }
    let Some(controller) = controller else {
        return Err(HostError::PluginFailed);
    };
    // Rewind the normalized stream — `getState` left the position at
    // the end and `setComponentState` reads from the start.
    // SAFETY: `norm_ptr` is a valid `IBStream`.
    state_result(unsafe { norm_ptr.seek(0, sdk_enum_i32(kIBSeekSet), ptr::null_mut()) })?;
    // SAFETY: `controller` is a live `IEditController`; `norm_ptr`
    // is a valid `IBStream` at position 0.
    state_result(unsafe { controller.setComponentState(norm_ptr.as_ptr()) })?;

    let ctrl_in = ComWrapper::new(BridgeStream::from_bytes(ctrl_bytes.to_vec()));
    let Some(ctrl_in_ptr) = ctrl_in.to_com_ptr::<IBStream>() else {
        return Err(HostError::PluginFailed);
    };
    // SAFETY: `controller` is live; `ctrl_in_ptr` holds `ctrl_bytes`
    // at position 0.
    state_result(unsafe { controller.setState(ctrl_in_ptr.as_ptr()) })
}

impl Plugin for Vst3Plugin {
    fn descriptor(&self) -> PluginDescriptor {
        self.descriptor
    }

    fn params(&self) -> &[ParamInfo] {
        self.params
    }

    /// `init` → `createInstance`(component CID) → `IComponent` +
    /// `IAudioProcessor` → `getControllerClassId` →
    /// `createInstance`(controller CID) — or `cast` on the component
    /// itself when it reports its own CID (single-component plugin)
    /// → `initialize(context)` (once for a shared object) →
    /// `setComponentHandler`. `create()` never instantiates.
    ///
    /// **RT contract:** non-RT only — instantiates the COM objects.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] on a second `init` or when any step
    /// of the chain declines; partially-initialized objects are
    /// best-effort `terminate`d and released before returning.
    #[expect(unsafe_code)]
    fn init(&mut self) -> Result<(), HostError> {
        if self.phase != Phase::Dormant {
            return Err(HostError::PluginFailed);
        }
        let Some(factory) = self.library.factory.as_ref() else {
            return Err(HostError::PluginFailed);
        };
        // SAFETY: `factory` is a live `IPluginFactory`; out-params are
        // valid; returned pointers become owned `ComPtr`s released on
        // scope exit — nothing leaks across an early return.
        unsafe {
            let Some(component) = create_instance::<IComponent>(factory, &self.cid) else {
                return Err(HostError::PluginFailed);
            };
            let Some(processor) = component.cast::<IAudioProcessor>() else {
                return Err(HostError::PluginFailed);
            };
            let mut controller_cid: TUID = [0; 16];
            if component.getControllerClassId(&raw mut controller_cid) != kResultOk {
                return Err(HostError::PluginFailed);
            }
            // A component reporting its own CID is a single-component
            // plugin — `IEditController` lives on the same object, so
            // `cast` resolves it. `createInstance` by the shared CID
            // would fabricate a disconnected controller clone.
            let single_component = controller_cid == self.cid;
            let Some(controller) = (if single_component {
                component.cast::<IEditController>()
            } else {
                create_instance::<IEditController>(factory, &controller_cid)
            }) else {
                return Err(HostError::PluginFailed);
            };
            let host_app = ComWrapper::new(BridgeHostApplication);
            let Some(context) = host_app.as_com_ref::<FUnknown>() else {
                return Err(HostError::PluginFailed);
            };
            if component.initialize(context.as_ptr()) != kResultOk {
                return Err(HostError::PluginFailed);
            }
            // A shared object is initialized once — a second
            // `initialize` on it may legitimately decline.
            if !single_component && controller.initialize(context.as_ptr()) != kResultOk {
                // The component reached `Initialized` — reverse it.
                let _ = component.terminate();
                return Err(HostError::PluginFailed);
            }
            let handler =
                ComWrapper::new(BridgeComponentHandler::new(Rc::clone(&self.dirty_notices)));
            let Some(handler_ptr) = handler.to_com_ptr::<IComponentHandler>() else {
                if !single_component {
                    let _ = controller.terminate();
                }
                let _ = component.terminate();
                return Err(HostError::PluginFailed);
            };
            if controller.setComponentHandler(handler_ptr.as_ptr()) != kResultOk {
                if !single_component {
                    let _ = controller.terminate();
                }
                let _ = component.terminate();
                return Err(HostError::PluginFailed);
            }
            // Distributed plugins: the spec makes the host connect the
            // component and controller `IConnectionPoint`s in both
            // directions — a controller that never sees its component
            // can legitimately refuse `createView`. Single-component
            // plugins expose no pair to connect.
            if !single_component {
                let component_cp = component.cast::<IConnectionPoint>();
                let controller_cp = controller.cast::<IConnectionPoint>();
                if let (Some(c_cp), Some(e_cp)) = (component_cp, controller_cp) {
                    if c_cp.connect(e_cp.as_ptr()) != kResultOk
                        || e_cp.connect(c_cp.as_ptr()) != kResultOk
                    {
                        let _ = e_cp.disconnect(c_cp.as_ptr());
                        let _ = c_cp.disconnect(e_cp.as_ptr());
                        let _ = controller.terminate();
                        let _ = component.terminate();
                        return Err(HostError::PluginFailed);
                    }
                    self.component_cp = Some(c_cp);
                    self.controller_cp = Some(e_cp);
                }
            }
            // `IMidiMapping` lives on the edit controller (a
            // single-component plugin answers through the same
            // object). `None` is honest — the plugin declares no
            // CC→parameter routing and every MIDI event keeps the
            // ordinary event path.
            let midi_mapping = controller.cast::<IMidiMapping>();
            self.component = Some(component);
            self.processor = Some(processor);
            self.controller = Some(controller);
            self.single_component = single_component;
            self.handler = Some(handler_ptr);
            self.midi_mapping = midi_mapping;
            // The host context must outlive the plugin objects — the
            // plugin may retain it past `initialize`.
            self.host_app = host_app.to_com_ptr::<IHostApplication>();
            self.phase = Phase::Initialized;
            Ok(())
        }
    }

    /// `start_processing` → `setProcessing(true)` (`Activated` →
    /// `Processing`).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] when the instance is not activated
    /// or `setProcessing` declines.
    #[expect(unsafe_code)]
    fn start_processing(&mut self) -> Result<(), HostError> {
        if self.phase != Phase::Activated {
            return Err(HostError::PluginFailed);
        }
        let Some(processor) = self.processor.as_ref() else {
            return Err(HostError::PluginFailed);
        };
        // SAFETY: `processor` is a live `IAudioProcessor`.
        if unsafe { processor.setProcessing(1) } == kResultOk {
            self.phase = Phase::Processing;
            Ok(())
        } else {
            Err(HostError::PluginFailed)
        }
    }

    /// `stop_processing` → `setProcessing(false)` (`Processing` →
    /// `Activated`).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] — advisory; the lifecycle has
    /// already left `Processing` or `setProcessing` declined.
    #[expect(unsafe_code)]
    fn stop_processing(&mut self) -> Result<(), HostError> {
        if self.phase != Phase::Processing {
            return Err(HostError::PluginFailed);
        }
        let Some(processor) = self.processor.as_ref() else {
            return Err(HostError::PluginFailed);
        };
        // SAFETY: `processor` is a live `IAudioProcessor`.
        if unsafe { processor.setProcessing(0) } == kResultOk {
            self.phase = Phase::Activated;
            Ok(())
        } else {
            Err(HostError::PluginFailed)
        }
    }

    /// `save_state` → `IComponent::getState` (+ `IEditController::
    /// getState` for distributed plugins) through bridge-owned
    /// `IBStream`s, framed as `[u32 LE comp_len][comp][u32 LE
    /// ctrl_len][ctrl]` (design
    /// `docs/designs/issue-630-adapter-state-bridge.md`).
    ///
    /// **RT contract:** non-RT only — the stream `Vec`s allocate
    /// inside this call and never touch the `process` path.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] before `init` or on a plugin
    /// state failure; [`HostError::Unsupported`] when the plugin
    /// answers `kNotImplemented` — the honest capability decline.
    fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
        if self.phase < Phase::Initialized {
            // Same bad-state convention as `open_gui` — the state
            // verbs are only legal on an initialized component.
            return Err(HostError::PluginFailed);
        }
        let Some(component) = self.component.as_ref() else {
            return Err(HostError::PluginFailed);
        };
        let (comp, ctrl) =
            save_component_state(component, self.controller.as_ref(), self.single_component)?;
        Ok(frame_state(&comp, &ctrl))
    }

    /// `restore_state` → the VST3-documented sequence
    /// `component.setState` → `component.getState` →
    /// `controller.setComponentState` → `controller.setState`
    /// (single-component plugins run the component pair only).
    ///
    /// **RT contract:** non-RT only — may allocate.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] before `init`, on a corrupt
    /// adapter frame, or on a plugin state failure;
    /// [`HostError::Unsupported`] on `kNotImplemented`. Blobs too
    /// short to be this adapter's framing are foreign state and are
    /// delegated to the plugin unchanged.
    fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
        if self.phase < Phase::Initialized {
            return Err(HostError::PluginFailed);
        }
        let Some(component) = self.component.as_ref() else {
            return Err(HostError::PluginFailed);
        };
        restore_component_state(
            component,
            self.controller.as_ref(),
            self.single_component,
            state,
        )
    }

    /// `open_gui` → `createView("editor")` →
    /// `isPlatformTypeSupported` → `setFrame(BridgePlugFrame)` →
    /// `attached(parent)` — Windows `HWND` only; other platforms and
    /// editor-less plugins answer `Unsupported` (design
    /// `docs/designs/issue-389-vst3-l0-editor-attach.md` D1/D4/D6).
    ///
    /// **Thread contract:** non-RT — must run on the thread owning
    /// the parent HWND's message pump (the L0 caller's UI thread);
    /// the adapter owns no pump and spawns no thread.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] when the instance is not yet
    /// `Initialized`, a view is already open, or a view-creation call
    /// declines; [`HostError::Unsupported`] when no editor view
    /// exists or the platform type is unsupported.
    #[expect(unsafe_code)]
    fn open_gui(&mut self, parent: ParentLease) -> Result<GuiHandle, HostError> {
        if self.phase < Phase::Initialized {
            // `createView` is only legal on an initialized
            // controller — the same bad-state convention as
            // `init`/`start_processing` misuse.
            return Err(HostError::PluginFailed);
        }
        if self.view.is_some() {
            // One live editor per instance — the session contract's
            // 1:1 rule; a second `open_gui` is a misuse, not a
            // capability answer.
            return Err(HostError::PluginFailed);
        }
        let Some(platform) = EDITOR_PLATFORM_TYPE else {
            return Err(HostError::Unsupported);
        };
        let Some(controller) = self.controller.as_ref() else {
            // `phase >= Initialized` implies `controller` — defensive
            // only (an empty `else` would hide a broken invariant).
            return Err(HostError::PluginFailed);
        };
        // SAFETY: `controller` is a live `IEditController` owned by
        // `self`; all calls run on the caller's UI thread per the doc
        // contract. `createView` returns an owned interface pointer
        // (or null) and COM objects released on scope exit below
        // leak nothing.
        let view = unsafe { ComPtr::from_raw(controller.createView(ViewType::kEditor)) };
        let Some(view) = view else {
            // `createView` null — the plugin ships no editor.
            return Err(HostError::Unsupported);
        };
        if unsafe { view.isPlatformTypeSupported(platform) } != kResultTrue {
            return Err(HostError::Unsupported);
        }
        let frame_wrapper = ComWrapper::new(BridgePlugFrame {
            request_sink: Rc::clone(&self.gui_request_sink),
        });
        let Some(frame) = frame_wrapper.to_com_ptr::<IPlugFrame>() else {
            return Err(HostError::PluginFailed);
        };
        // `parent.payload()` is the redeemed raw platform handle — re-widened
        // inside the OS adapter boundary only; `generation` is issuer
        // metadata the OS layer never interprets.
        let parent_ptr = parent.payload() as *mut c_void;
        if unsafe { view.setFrame(frame.as_ptr()) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        if unsafe { view.attached(parent_ptr, platform) } != kResultOk {
            let _ = unsafe { view.setFrame(ptr::null_mut()) };
            return Err(HostError::PluginFailed);
        }
        self.gui_seq = self.gui_seq.wrapping_add(1);
        self.plug_frame = Some(frame);
        self.gui_parent = Some((parent, platform));
        self.gui_attached = true;
        self.view = Some(view);
        Ok(GuiHandle::new(self.gui_seq))
    }

    /// `close_gui` → `removed()` → release view + frame.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open — closing an
    /// unopened GUI is not a no-op success.
    fn close_gui(&mut self) -> Result<(), HostError> {
        if self.view.is_none() {
            return Err(HostError::Unsupported);
        }
        self.close_view();
        Ok(())
    }

    /// `set_gui_size` → `canResize` gate → `checkSizeConstraint` →
    /// `onSize` — the plugin is asked whether it can resize, its
    /// constraint hook may adjust the rect, and `onSize` carries the
    /// final size. `GuiSize` logical units map 1:1 onto `ViewRect`
    /// pixels — the same documented unit deviation the attach path
    /// already carries (DPI scaling is a later concern).
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open or the plugin
    /// declines resize via `canResize`; [`HostError::PluginFailed`]
    /// when a view call declines.
    #[expect(unsafe_code)]
    #[expect(clippy::cast_possible_truncation)]
    fn set_gui_size(&mut self, size: GuiSize) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        // SAFETY: `view` is a live `IPlugView`; all calls run on the
        // caller's UI thread per the open_gui thread contract.
        if unsafe { view.canResize() } != kResultTrue {
            return Err(HostError::Unsupported);
        }
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: size.width.round() as i32,
            bottom: size.height.round() as i32,
        };
        if unsafe { view.checkSizeConstraint(&raw mut rect) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        if unsafe { view.onSize(&raw mut rect) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        Ok(())
    }

    /// `set_gui_visibility` — `IPlugView` has no hide/show verb, so
    /// hide is `removed()` and show is `setFrame` + `attached`
    /// re-composition under the stored parent: the plugin's editor
    /// surface is genuinely detached/reattached (UI state held by the
    /// plugin's window is lost on hide — documented in the design).
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open or this
    /// platform has no implemented GUI API; [`HostError::PluginFailed`]
    /// when a view call declines.
    #[expect(unsafe_code)]
    fn set_gui_visibility(&mut self, visible: bool) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        let Some((parent, platform)) = self.gui_parent else {
            return Err(HostError::PluginFailed);
        };
        if visible == self.gui_attached {
            return Ok(());
        }
        if !visible {
            // SAFETY: live `IPlugView` on the caller's UI thread.
            if unsafe { view.removed() } != kResultOk {
                return Err(HostError::PluginFailed);
            }
            self.gui_attached = false;
            return Ok(());
        }
        if let Some(frame) = self.plug_frame.as_ref()
            && unsafe { view.setFrame(frame.as_ptr()) } != kResultOk
        {
            return Err(HostError::PluginFailed);
        }
        let parent_ptr = parent.payload() as *mut c_void;
        // SAFETY: live `IPlugView` + live parent HWND on the caller's
        // UI thread — the same contract `open_gui` runs under.
        if unsafe { view.attached(parent_ptr, platform) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        self.gui_attached = true;
        Ok(())
    }

    /// `set_gui_parent` → `removed()` + `attached(new parent)` —
    /// re-parents the open view under a freshly redeemed lease. On an
    /// attach failure the old parent is re-attached best-effort so a
    /// declined re-parent never leaves the view detached.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open or this
    /// platform has no implemented GUI API; [`HostError::PluginFailed`]
    /// when a view call declines.
    #[expect(unsafe_code)]
    fn set_gui_parent(&mut self, parent: ParentLease) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        let Some(platform) = EDITOR_PLATFORM_TYPE else {
            return Err(HostError::Unsupported);
        };
        let previous = self.gui_parent;
        if self.gui_attached && unsafe { view.removed() } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        self.gui_attached = false;
        let parent_ptr = parent.payload() as *mut c_void;
        // SAFETY: live `IPlugView` + redeemed parent HWND on the
        // caller's UI thread — the `open_gui` attach contract.
        if unsafe { view.attached(parent_ptr, platform) } != kResultOk {
            // Best-effort restore: a declined re-parent must not
            // strand the view detached under a dead lease.
            if let Some((old_parent, _)) = previous {
                let _ = unsafe { view.attached(old_parent.payload() as *mut c_void, platform) };
            }
            return Err(HostError::PluginFailed);
        }
        self.gui_parent = Some((parent, platform));
        self.gui_attached = true;
        Ok(())
    }

    /// `set_gui_focus` → `IPlugView::onFocus` — forwards the host's
    /// focus flag as the VST3 `TBool` state.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open;
    /// [`HostError::PluginFailed`] when the view declines.
    #[expect(unsafe_code)]
    fn set_gui_focus(&mut self, focused: bool) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        // SAFETY: live `IPlugView` on the caller's UI thread — the
        // same contract `open_gui` runs under.
        if unsafe { view.onFocus(TBool::from(focused)) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        Ok(())
    }

    /// `send_gui_key` → `IPlugView::onKeyDown`/`onKeyUp` — the
    /// boundary `GuiKeyEvent` maps onto the VST3 triple: `key` is the
    /// UTF-16 `char16`, `key_code` the VST3 virtual key code, and
    /// `modifiers` the `KeyModifier` bitmask.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open;
    /// [`HostError::InvalidValue`] when `key` does not fit one UTF-16
    /// code unit; [`HostError::PluginFailed`] when the view declines.
    #[expect(unsafe_code)]
    fn send_gui_key(&mut self, event: GuiKeyEvent) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        // `char16` is one UTF-16 code unit — a supplementary-plane
        // `key` cannot be truncated into a surrogate pair the verb
        // does not carry.
        let key = u16::try_from(u32::from(event.key)).map_err(|_| HostError::InvalidValue)?;
        let modifiers = key_modifiers(event.modifiers);
        // SAFETY: live `IPlugView` on the caller's UI thread — the
        // same contract `open_gui` runs under.
        let result = unsafe {
            match event.direction {
                GuiKeyDirection::Down => view.onKeyDown(key, event.key_code, modifiers),
                GuiKeyDirection::Up => view.onKeyUp(key, event.key_code, modifiers),
                // `GuiKeyDirection` is `#[non_exhaustive]` — an
                // unknown direction has no defined VST3 verb.
                _ => return Err(HostError::InvalidValue),
            }
        };
        if result != kResultOk {
            return Err(HostError::PluginFailed);
        }
        Ok(())
    }

    /// `set_gui_focus_target` — the target-parameterized focus verb
    /// (Issue #630 F2). `Editor` is the same destination
    /// `set_gui_focus(true)` names (`onFocus(1)`); `EditorElement`
    /// rides the ruviform-private `IRvfEditorOps` seam — delivery
    /// semantics only, not an OS-focus claim; `HostWindow` moves OS
    /// focus back to the session container HWND stored at attach and
    /// notifies the view via `onFocus(false)` (Windows only — the
    /// same UI-thread contract `open_gui` runs under).
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open, the private
    /// seam is absent, `HostWindow` has no stored parent or runs
    /// off-Windows; [`HostError::PluginFailed`] when a view/seam call
    /// or the container `SetFocus` declines; [`HostError::InvalidValue`]
    /// for an unknown `#[non_exhaustive]` target.
    #[expect(unsafe_code)]
    fn set_gui_focus_target(&mut self, target: GuiFocusTarget) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        match target {
            GuiFocusTarget::Editor => {
                // SAFETY: live `IPlugView` on the caller's UI thread —
                // the same contract `open_gui` runs under.
                if unsafe { view.onFocus(TBool::from(true)) } != kResultOk {
                    return Err(HostError::PluginFailed);
                }
                Ok(())
            }
            GuiFocusTarget::EditorElement { element } => {
                let Some(ops) = view.cast::<IRvfEditorOps>() else {
                    return Err(HostError::Unsupported);
                };
                // SAFETY: `ops` is a live seam pointer QI'd from the
                // open view on the caller's UI thread.
                if unsafe { ops.apply_focus(element, TBool::from(true)) } != kResultOk {
                    return Err(HostError::PluginFailed);
                }
                Ok(())
            }
            GuiFocusTarget::HostWindow => self.focus_host_window(view),
            // `GuiFocusTarget` is `#[non_exhaustive]` — an unknown
            // target has no defined VST3 destination.
            _ => Err(HostError::InvalidValue),
        }
    }

    /// `set_gui_popup` — plugin-owned popup drive through the same
    /// `IRvfEditorOps` seam (Issue #630 P1): `Dialog` reaches the
    /// fixture's `WS_POPUP` surface, `ChildWindow` its `WS_CHILD`
    /// auxiliary surface. Explicit-state and idempotent — the fixture
    /// records transitions only.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] when no view is open or the seam is
    /// absent; [`HostError::PluginFailed`] when the seam call
    /// declines; [`HostError::InvalidValue`] for an unknown
    /// `#[non_exhaustive]` kind.
    #[expect(unsafe_code)]
    fn set_gui_popup(&mut self, kind: GuiPopupKind, open: bool) -> Result<(), HostError> {
        let Some(view) = self.view.as_ref() else {
            return Err(HostError::Unsupported);
        };
        let kind_code = match kind {
            GuiPopupKind::ChildWindow => POPUP_KIND_CHILD_WINDOW,
            GuiPopupKind::Dialog => POPUP_KIND_DIALOG,
            // `GuiPopupKind` is `#[non_exhaustive]` — an unknown kind
            // has no defined seam code point.
            _ => return Err(HostError::InvalidValue),
        };
        let Some(ops) = view.cast::<IRvfEditorOps>() else {
            return Err(HostError::Unsupported);
        };
        // SAFETY: `ops` is a live seam pointer QI'd from the open
        // view on the caller's UI thread.
        if unsafe { ops.popup(kind_code, TBool::from(open)) } != kResultOk {
            return Err(HostError::PluginFailed);
        }
        Ok(())
    }

    /// `set_gui_request_sink` — stores the session-installed route
    /// in the shared cell `BridgePlugFrame` reads on `resizeView`
    /// (Issue #873). `try_borrow_mut` is the defined guard against a
    /// re-entrant install while a request is in flight.
    fn set_gui_request_sink(&mut self, sink: Option<Box<GuiRequestSink>>) {
        if let Ok(mut slot) = self.gui_request_sink.try_borrow_mut() {
            *slot = sink;
        }
    }

    /// `drain_dirty_notices` — moves the shared queue's pending
    /// plugin→host dirty notifications into `out` (Issue #891). A
    /// `borrow_mut` conflict means a plugin handler call is in flight
    /// on this thread — the queued notices survive for the next pull,
    /// so the drain yields nothing rather than panicking.
    fn drain_dirty_notices(&mut self, out: &mut Vec<DirtyNotice>) {
        if let Ok(mut queue) = self.dirty_notices.try_borrow_mut() {
            queue.drain(out);
        }
    }
}

/// Maps the boundary [`GuiModifiers`] bitmask onto the VST3
/// `KeyModifier` bitmask (`int16`) — the two layouts do not share
/// bit positions, so the translation is per-flag, not a passthrough.
// `kShiftKey`..`kControlKey` are the binding's wider enum type
// (`KeyModifier = DefaultEnumType`) but the defined VST3 modifier
// bits are 1/2/4/8 — every value fits `int16`, which is exactly
// what `onKeyDown`/`onKeyUp` take.
#[expect(clippy::cast_possible_truncation)]
fn key_modifiers(modifiers: GuiModifiers) -> i16 {
    let mut bits: i16 = 0;
    if modifiers.contains(GuiModifiers::SHIFT) {
        bits |= kShiftKey as i16;
    }
    if modifiers.contains(GuiModifiers::ALTERNATE) {
        bits |= kAlternateKey as i16;
    }
    if modifiers.contains(GuiModifiers::COMMAND) {
        bits |= kCommandKey as i16;
    }
    if modifiers.contains(GuiModifiers::CONTROL) {
        bits |= kControlKey as i16;
    }
    bits
}

// ---------------------------------------------------------------------------
// Editor view platform type
// ---------------------------------------------------------------------------

/// The VST3 platform type string for this OS — `Some` only on
/// Windows (`kPlatformTypeHWND`); other platforms hold `None` so
/// `open_gui` answers the honest `Unsupported` instead of claiming an
/// unverified `NSView`/`X11EmbedWindowID` attach (design
/// `docs/designs/issue-389-vst3-l0-editor-attach.md` D4).
#[cfg(target_os = "windows")]
const EDITOR_PLATFORM_TYPE: Option<vst3::Steinberg::FIDString> =
    Some(vst3::Steinberg::kPlatformTypeHWND);

/// Non-Windows attach is not implemented — see the Windows arm.
#[cfg(not(target_os = "windows"))]
const EDITOR_PLATFORM_TYPE: Option<vst3::Steinberg::FIDString> = None;

// ---------------------------------------------------------------------------
// Win32 focus FFI (GuiFocusTarget::HostWindow)
// ---------------------------------------------------------------------------

/// The two user32 verbs the `HostWindow` focus handoff needs —
/// declared bare because this crate carries no `windows-sys` edge:
/// the adapter's entire Win32 surface is this one handoff, so a
/// two-symbol `#[link]` block is the smallest honest footprint.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
mod win32 {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        /// `SetFocus(hwnd)` — moves keyboard focus to `hwnd` inside
        /// the calling thread's input queue.
        fn SetFocus(hwnd: *mut c_void) -> *mut c_void;
        /// `GetFocus()` — the calling thread's focused HWND or null.
        fn GetFocus() -> *mut c_void;
    }

    /// `SetFocus(hwnd)` verified through the `GetFocus()` oracle —
    /// `true` only when the calling thread's focus actually landed on
    /// `hwnd`. The `SetFocus` return is deliberately ignored: it is
    /// the previously focused HWND *or* null, which success also
    /// produces when nothing held focus before.
    pub(super) fn set_focus(hwnd: *mut c_void) -> bool {
        // SAFETY: `hwnd` is the container HWND the session attached
        // us under, on the UI thread that owns it — a dead or
        // foreign-thread handle makes `SetFocus` fail and the oracle
        // answers `false`, which the caller maps to `PluginFailed`.
        unsafe {
            SetFocus(hwnd);
            GetFocus() == hwnd
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture resolution helper (shared with the Task-3 runner / Task-4 E2E)
// ---------------------------------------------------------------------------

/// The workspace-built VST3 fixture's dynamic-library file name:
/// `{DLL_PREFIX}ruviform_plugin_vst3{DLL_SUFFIX}` — the same
/// `current_exe`-relative convention the L1 runner and the contract
/// tests resolve by.
#[must_use]
pub fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

// ---------------------------------------------------------------------------
// Tests — in-process dlopen of the workspace-built fixture cdylib
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "host"))]
mod tests {
    use super::*;
    use crate::component::GainComponent;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_media_core::{
        MediaTime, MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiValue,
        MusicalTimeInfo, NoteKey,
    };
    // `ParamValue` here is the *neutral* normalized-value type — the
    // named import shadows the `vst3` `f64` alias brought in by
    // `use super::*`.
    use ruviform_plugin_core::{
        Event, EventKind, GuiRequestRefusal, HostInput, HostOutput, ParamEvent, ParamValue,
    };
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::rc::Rc;
    use vst3::ComRef;
    use vst3::Steinberg::Vst::RoutingInfo;

    const FRAMES: usize = 64;
    const FRAMES_U32: u32 = 64;

    /// `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_vst3{DLL_SUFFIX}`
    /// — the test spec's fixture resolution; a missing binary is an
    /// `assert!` failure, never a silent skip.
    fn fixture_path() -> PathBuf {
        let exe = std::env::current_exe().expect("current_exe is available");
        let deps = exe.parent().expect("test binary lives in deps/");
        deps.join(fixture_lib_name())
    }

    fn load_fixture() -> Vst3Library {
        let path = fixture_path();
        assert!(path.is_file(), "fixture cdylib missing: {}", path.display());
        Vst3Library::load(&path).expect("fixture dlopen succeeds")
    }

    /// The expected descriptor id — derived at test time from the raw
    /// `GainComponent::CID` bytes (the `uid()` platform-order layout),
    /// independently of the production `tuid_hex` so the hex mapping
    /// is verified, not restated.
    fn expected_id() -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(32);
        for &byte in &GainComponent::CID {
            let byte = byte.to_ne_bytes()[0];
            out.push(char::from(HEX[(byte >> 4) as usize]));
            out.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
        out
    }

    /// Stereo input pair `0.5 / -0.25` through the fixture and back.
    fn drive_block(
        plugin: &mut dyn ErasedPlugin,
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> (ProcessResult, [Vec<f32>; 2]) {
        let in_left = vec![0.5_f32; FRAMES];
        let in_right = vec![-0.25_f32; FRAMES];
        let in_channels = [&in_left[..], &in_right[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        let result = {
            let mut out_channels = [&mut out_left[..], &mut out_right[..]];
            let mut out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();
            let inputs = [HostInput(&in_view)];
            let mut outputs = [HostOutput(&mut out_view)];
            plugin.process_host(
                &inputs,
                &mut outputs,
                events,
                transport,
                &mut ProcessContext::new(),
            )
        };
        (result, [out_left, out_right])
    }

    fn prepared_plugin() -> Box<dyn ErasedPlugin> {
        let library = load_fixture();
        let factories = library.factories();
        assert_eq!(factories.len(), 1);
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
        plugin
            .prepare(&config, &mut ProcessContext::new())
            .expect("prepare");
        plugin.start_processing().expect("start_processing");
        plugin
    }

    #[test]
    fn descriptors_match_fixture_known_values() {
        let library = load_fixture();
        let descriptors = library.descriptors();
        // Exactly one descriptor — the controller class is filtered
        // out (the module declares 2 classes: `Gain` +
        // `Gain Controller`).
        assert_eq!(descriptors.len(), 1);
        let descriptor = descriptors[0];
        let expected = expected_id();
        // The `uid()` byte order is platform-dependent: Windows
        // reverses the first GUID fields; other platforms are
        // big-endian. The literal is a regression pin only — the real
        // check is the `GainComponent::CID` byte derivation.
        #[cfg(target_os = "windows")]
        assert_eq!(expected, "3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(expected, "5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d");
        assert_eq!(descriptor.id, expected);
        assert_eq!(descriptor.name, "Gain");
        assert_eq!(descriptor.vendor, "Ruviform");
        assert_eq!(descriptor.version, "0.0.0");
        // `subCategories` order is preserved.
        assert_eq!(descriptor.features, ["Fx"]);
    }

    #[test]
    fn controller_class_is_not_a_plugin() {
        // `"Component Controller Class"` entries pair with components
        // via `getControllerClassId` — they are never standalone
        // plugins. One class scanned, one factory produced.
        let library = load_fixture();
        assert_eq!(library.descriptors().len(), 1);
        assert_eq!(library.descriptors()[0].name, "Gain");
        assert_eq!(library.factories().len(), 1);
    }

    #[test]
    fn descriptors_are_cached_across_calls() {
        let library = load_fixture();
        let first = library.descriptors();
        let second = library.descriptors();
        // The intern-once cached slice — same pointer, same values, no
        // per-call scan or leak growth.
        assert!(ptr::eq(first.as_ptr(), second.as_ptr()));
        assert_eq!(first, second);
    }

    #[test]
    fn load_missing_path_returns_error() {
        let result = Vst3Library::load("definitely/not/a/plugin.vst3");
        assert!(matches!(result, Err(Vst3HostError::Library(_))));
    }

    #[test]
    fn load_non_vst3_binary_returns_error() {
        // The running test binary is a loadable image without
        // `GetPluginFactory` — a defined `Err`, never a panic.
        let exe = std::env::current_exe().expect("current_exe");
        assert!(matches!(
            Vst3Library::load(&exe),
            Err(Vst3HostError::MissingEntryPoint(_) | Vst3HostError::Library(_))
        ));
    }

    #[test]
    fn factory_reports_cached_descriptor_and_no_sendable() {
        let library = load_fixture();
        let factories = library.factories();
        assert_eq!(factories.len(), 1);
        let factory = &factories[0];
        assert_eq!(factory.descriptor(), library.descriptors()[0]);
        // COM objects hold raw pointers (`!Send`) — the opt-in stays
        // `None`.
        assert!(factory.create_sendable().is_none());
    }

    #[test]
    fn params_table_matches_fixture_at_every_lifecycle_point() {
        let library = load_fixture();
        let factories = library.factories();
        let plugin = factories[0].create();
        // Pre-init — the eager factory-time enumeration already caches
        // the table (lazy enumeration could not answer here).
        let params = plugin.params();
        assert_eq!(params.len(), 1);
        let param = params[0];
        assert_eq!(param.id, ParamId(0));
        assert_eq!(param.name, "Gain");
        // `ParameterInfo.units` — the fixture declares `""`.
        assert_eq!(param.unit, "");
        assert_eq!(param.min_value, 0.0);
        assert_eq!(param.max_value, 1.0);
        assert_eq!(param.default_value, 1.0);
        // `kCanAutomate` → `AUTOMATABLE`.
        assert!(param.flags.contains(ParamFlags::AUTOMATABLE));

        // Post-init and activated — still the same cached table.
        let mut plugin = prepared_plugin();
        assert_eq!(plugin.params()[0], param);
        plugin.stop_processing().expect("stop_processing");
        assert_eq!(plugin.params()[0], param);
    }

    #[test]
    fn lifecycle_roundtrip_processes_passthrough_block() {
        let mut plugin = prepared_plugin();
        let (result, [left, right]) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32), None);
        assert_eq!(result.status, ProcessStatus::Continue);
        // Default gain 1.0 — input carried verbatim.
        assert_eq!(left, vec![0.5_f32; FRAMES]);
        assert_eq!(right, vec![-0.25_f32; FRAMES]);
        plugin.stop_processing().expect("stop_processing");
        // `drop` runs the symmetric teardown — stop (already stopped),
        // setActive(false), terminate on both objects, release.
    }

    #[test]
    fn param_event_reaches_plugin_normalized() {
        let mut plugin = prepared_plugin();
        // The fixture's normalized/plain domains coincide (identity
        // mapping) — normalized 0.25 → gain 0.25.
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.25).unwrap(),
            }),
        )];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, right]) = drive_block(&mut *plugin, events, None);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(left, vec![0.125_f32; FRAMES]);
        assert_eq!(right, vec![-0.0625_f32; FRAMES]);
    }

    /// Canonical `EventKind::Midi` events translate into
    /// `ProcessData.inputEvents` and the fixture's plugin-side
    /// `collect_incoming` drains them without rejecting the block —
    /// the gain fixture ignores MIDI, so audio stays a passthrough.
    /// Translation correctness itself is covered by `midi::tests`
    /// (both directions + the `IEventList` COM round trip).
    #[test]
    fn midi_events_reach_process_as_input_events() {
        let mut plugin = prepared_plugin();
        let note_on = MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        };
        let events = [Event::new(0, EventKind::Midi(note_on))];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, right]) = drive_block(&mut *plugin, events, None);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(left, vec![0.5_f32; FRAMES]);
        assert_eq!(right, vec![-0.25_f32; FRAMES]);
    }

    #[test]
    fn process_before_start_is_defined_error() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
        plugin
            .prepare(&config, &mut ProcessContext::new())
            .expect("prepare");
        // Activated but never started — defined rejection, no panic.
        let (result, _) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32), None);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    #[test]
    fn oversized_block_is_defined_error() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        // Negotiate 32 frames, then drive a 64-frame block.
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 32).unwrap();
        plugin
            .prepare(&config, &mut ProcessContext::new())
            .expect("prepare");
        plugin.start_processing().expect("start_processing");
        let (result, _) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32), None);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    #[test]
    fn drop_mid_processing_completes_teardown() {
        // Dropping while `Processing` runs the pinned teardown:
        // `setProcessing(false)` first (the fixture rejects
        // `setActive(false)` while processing), then
        // `setActive(false)`/`terminate`. `Vst3Plugin::drop` asserts
        // each step's tresult via `debug_assert_eq!` — a wrong order
        // makes the fixture answer `kResultFalse`, which panics here
        // instead of passing silently.
        let mut plugin = prepared_plugin();
        let (result, _) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32), None);
        assert_eq!(result.status, ProcessStatus::Continue);
        drop(plugin);
    }

    #[test]
    fn state_verbs_gate_and_fixture_declines() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        // Pre-init — the bad-state refusal, same convention as
        // `open_gui` (phase < Initialized → `PluginFailed`).
        assert_eq!(plugin.save_state(), Err(HostError::PluginFailed));
        assert_eq!(
            plugin.restore_state(&[1, 2, 3]),
            Err(HostError::PluginFailed)
        );
        plugin.init().expect("init");
        // The workspace fixture's state verbs are honest
        // `kNotImplemented` stubs → the defined capability decline
        // (`Unsupported`), never a fabricated success — the answer
        // the L1 e2e pins over the wire.
        assert_eq!(plugin.save_state(), Err(HostError::Unsupported));
        // `[1, 2, 3]` is shorter than the 8-byte frame header, so it
        // is foreign state delegated to the plugin — which declines
        // `kNotImplemented` → `Unsupported`.
        assert_eq!(
            plugin.restore_state(&[1, 2, 3]),
            Err(HostError::Unsupported)
        );
        // A blob long enough to be this adapter's framing but corrupt
        // (`comp_len` exceeds the payload) is rejected before any
        // plugin call → `PluginFailed`.
        let mut corrupt = Vec::new();
        corrupt.extend_from_slice(&100u32.to_le_bytes());
        corrupt.extend_from_slice(b"short");
        assert_eq!(plugin.restore_state(&corrupt), Err(HostError::PluginFailed));
    }

    #[test]
    fn open_gui_before_init_is_plugin_failed() {
        // `phase=Dormant` — `createView` is only legal on an
        // initialized controller, so the pre-init call is the
        // bad-state refusal (`PluginFailed`), not a capability answer
        // (test-spec VA-02).
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        assert_eq!(
            plugin.open_gui(ParentLease::new(0, 0)),
            Err(HostError::PluginFailed)
        );
        // close with nothing open is the honest refusal.
        assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));
    }

    #[test]
    fn open_gui_null_view_is_unsupported() {
        // The workspace fixture's `createView` returns null (no
        // editor shipped) — the adapter maps that to the honest
        // `Unsupported`, not a fabricated success (VA-03).
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        assert_eq!(
            plugin.open_gui(ParentLease::new(0, 0)),
            Err(HostError::Unsupported)
        );
        // The failed open leaves no view: close is the same refusal.
        assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));
        // `drop` now runs `close_view` first — a defined no-op here,
        // then the pinned descent (Initialized → terminate).
    }

    // -------------------------------------------------------------------
    // `IRvfEditorOps` seam verbs — fabricated views over a real library
    // -------------------------------------------------------------------

    /// A fabricated `IPlugView` that also answers the private
    /// `IRvfEditorOps` seam — records each delivery so the verb path
    /// through `Vst3Plugin` is provable headlessly. `decline` makes
    /// the seam verbs answer `kResultFalse` → the adapter's
    /// `PluginFailed` mapping.
    struct SeamView {
        events: RefCell<Vec<String>>,
        decline: Cell<bool>,
    }

    impl Class for SeamView {
        type Interfaces = (IPlugView, IRvfEditorOps);
    }

    macro_rules! view_stubs {
        () => {
            #[expect(unsafe_code)]
            unsafe fn isPlatformTypeSupported(&self, _type: vst3::Steinberg::FIDString) -> tresult {
                kResultTrue
            }
            #[expect(unsafe_code)]
            unsafe fn attached(
                &self,
                _parent: *mut c_void,
                _type: vst3::Steinberg::FIDString,
            ) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn removed(&self) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn onWheel(&self, _distance: f32) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn onKeyDown(&self, _key: u16, _key_code: i16, _mods: i16) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn onKeyUp(&self, _key: u16, _key_code: i16, _mods: i16) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn getSize(&self, _size: *mut ViewRect) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn onSize(&self, _new_size: *mut ViewRect) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn setFrame(&self, _frame: *mut IPlugFrame) -> tresult {
                kResultOk
            }
            #[expect(unsafe_code)]
            unsafe fn canResize(&self) -> tresult {
                kResultTrue
            }
            #[expect(unsafe_code)]
            unsafe fn checkSizeConstraint(&self, _rect: *mut ViewRect) -> tresult {
                kResultOk
            }
        };
    }

    impl IPlugViewTrait for SeamView {
        view_stubs!();
        #[expect(unsafe_code)]
        unsafe fn onFocus(&self, state: TBool) -> tresult {
            self.events.borrow_mut().push(format!("f:{state}"));
            kResultOk
        }
    }

    impl IRvfEditorOpsTrait for SeamView {
        #[expect(unsafe_code)]
        unsafe fn apply_focus(&self, element: u32, focused: TBool) -> tresult {
            if self.decline.get() {
                return kResultFalse;
            }
            self.events
                .borrow_mut()
                .push(format!("fe:{element}:{focused}"));
            kResultOk
        }
        #[expect(unsafe_code)]
        unsafe fn popup(&self, kind: u32, open: TBool) -> tresult {
            if self.decline.get() {
                return kResultFalse;
            }
            self.events
                .borrow_mut()
                .push(format!("popup:{kind}:{open}"));
            kResultOk
        }
    }

    /// A fabricated `IPlugView` WITHOUT the seam — the honest
    /// no-capability answer the element/popup arms must map to
    /// `Unsupported`.
    struct PlainView {
        events: RefCell<Vec<String>>,
    }

    impl Class for PlainView {
        type Interfaces = (IPlugView,);
    }

    impl IPlugViewTrait for PlainView {
        view_stubs!();
        #[expect(unsafe_code)]
        unsafe fn onFocus(&self, state: TBool) -> tresult {
            self.events.borrow_mut().push(format!("f:{state}"));
            kResultOk
        }
    }

    /// A `Vst3Plugin` whose `view` is a fabricated `ComWrapper` view —
    /// bypasses `open_gui` (the workspace fixture's `createView`
    /// returns null) so the seam verb paths are exercisable
    /// headlessly. The returned `ComWrapper` keeps the class data
    /// alive for event inspection.
    fn plugin_with_view<C: Class>(wrapper: &ComWrapper<C>) -> Vst3Plugin {
        let library = load_fixture();
        let factories = library.factories();
        let factory = &factories[0];
        let mut plugin = Vst3Plugin::dormant(
            factory.descriptor,
            factory.params,
            factory.cid,
            Arc::clone(&factory.library),
        );
        plugin.view = wrapper.to_com_ptr::<IPlugView>();
        plugin
    }

    /// `set_gui_focus_target(Editor)` reaches `onFocus(1)` — the same
    /// destination `set_gui_focus(true)` names.
    #[test]
    fn focus_target_editor_uses_on_focus() {
        let wrapper = ComWrapper::new(SeamView {
            events: RefCell::new(Vec::new()),
            decline: Cell::new(false),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(plugin.set_gui_focus_target(GuiFocusTarget::Editor), Ok(()));
        assert_eq!(wrapper.events.borrow().as_slice(), ["f:1"]);
    }

    /// `set_gui_focus_target(EditorElement)` QI'd the seam and
    /// delivered `apply_focus(element, 1)` — delivery semantics, not
    /// an OS-focus claim.
    #[test]
    fn focus_target_element_delivers_seam() {
        let wrapper = ComWrapper::new(SeamView {
            events: RefCell::new(Vec::new()),
            decline: Cell::new(false),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 7 }),
            Ok(())
        );
        assert_eq!(wrapper.events.borrow().as_slice(), ["fe:7:1"]);
    }

    /// `set_gui_popup` maps both `GuiPopupKind`s onto the seam's
    /// `popup(kind, open)` code points.
    #[test]
    fn popup_verbs_deliver_seam_kind_codes() {
        let wrapper = ComWrapper::new(SeamView {
            events: RefCell::new(Vec::new()),
            decline: Cell::new(false),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(plugin.set_gui_popup(GuiPopupKind::Dialog, true), Ok(()));
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::ChildWindow, true),
            Ok(())
        );
        assert_eq!(plugin.set_gui_popup(GuiPopupKind::Dialog, false), Ok(()));
        assert_eq!(
            wrapper.events.borrow().as_slice(),
            [
                format!("popup:{POPUP_KIND_DIALOG}:1"),
                format!("popup:{POPUP_KIND_CHILD_WINDOW}:1"),
                format!("popup:{POPUP_KIND_DIALOG}:0"),
            ]
        );
    }

    /// A view without the seam answers the defined `Unsupported` —
    /// `Editor` still works through plain `onFocus`, so the refusal
    /// is scoped to the seam verbs exactly.
    #[test]
    fn seamless_view_declines_element_and_popups() {
        let wrapper = ComWrapper::new(PlainView {
            events: RefCell::new(Vec::new()),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 1 }),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::Dialog, true),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::ChildWindow, true),
            Err(HostError::Unsupported)
        );
        // The non-seam arm is unaffected.
        assert_eq!(plugin.set_gui_focus_target(GuiFocusTarget::Editor), Ok(()));
        assert_eq!(wrapper.events.borrow().as_slice(), ["f:1"]);
    }

    /// A seam verb the plugin declines (`kResultFalse`) maps to
    /// `PluginFailed` — the plugin's refusal, never a fabricated
    /// success.
    #[test]
    fn declining_seam_is_plugin_failed() {
        let wrapper = ComWrapper::new(SeamView {
            events: RefCell::new(Vec::new()),
            decline: Cell::new(true),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 1 }),
            Err(HostError::PluginFailed)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::Dialog, true),
            Err(HostError::PluginFailed)
        );
    }

    /// No open view — every new verb answers `Unsupported`, the same
    /// gate `set_gui_focus`/`send_gui_key` carry.
    #[test]
    fn editor_verbs_without_view_are_unsupported() {
        let library = load_fixture();
        let factories = library.factories();
        let factory = &factories[0];
        let mut plugin = Vst3Plugin::dormant(
            factory.descriptor,
            factory.params,
            factory.cid,
            Arc::clone(&factory.library),
        );
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::Editor),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::EditorElement { element: 1 }),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::HostWindow),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.set_gui_popup(GuiPopupKind::Dialog, true),
            Err(HostError::Unsupported)
        );
    }

    /// `HostWindow` without a stored parent lease is `Unsupported`
    /// on every OS; on Windows a dead lease payload fails the
    /// `GetFocus` oracle → `PluginFailed` (off-Windows the whole arm
    /// is `Unsupported` before any OS call).
    #[test]
    fn host_window_focus_requires_a_stored_parent() {
        let wrapper = ComWrapper::new(SeamView {
            events: RefCell::new(Vec::new()),
            decline: Cell::new(false),
        });
        let mut plugin = plugin_with_view(&wrapper);
        assert_eq!(
            plugin.set_gui_focus_target(GuiFocusTarget::HostWindow),
            Err(HostError::Unsupported)
        );

        #[cfg(target_os = "windows")]
        {
            // A stored lease whose payload is not a live HWND — the
            // `GetFocus` oracle answers `false` → `PluginFailed`, and
            // the plugin sees no `onFocus(false)` notification.
            plugin.gui_parent = Some((
                ParentLease::new(0, 0xDEAD),
                vst3::Steinberg::kPlatformTypeHWND,
            ));
            assert_eq!(
                plugin.set_gui_focus_target(GuiFocusTarget::HostWindow),
                Err(HostError::PluginFailed)
            );
            assert!(wrapper.events.borrow().is_empty());
        }
    }

    #[test]
    fn prepare_rejects_frames_above_wire_ceiling() {
        // `max_frames` sizes the per-instance scratch — above the wire
        // protocol's `WIRE_MAX_FRAMES` ceiling `prepare` answers a
        // defined `UnsupportedConfig` instead of allocating.
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        let config =
            ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, WIRE_MAX_FRAMES + 1).unwrap();
        assert_eq!(
            plugin.prepare(&config, &mut ProcessContext::new()),
            Err(PrepareError::UnsupportedConfig)
        );
    }

    #[test]
    fn transport_snapshot_maps_to_process_context() {
        // The `Some(transport)` arm of `map_process_context` — every
        // other test drives `None`, so the mapping is pinned
        // field-by-field here. A `MediaTime` already in the 48 kHz
        // base converts 1:1; PPQ fields are `ticks / 960`; the flag
        // set is asserted exactly (no `kContTimeValid`).
        let base = TimeBase::sample_rate(48_000).unwrap();
        let transport = TransportInfo::new(TransportStatus::Playing, MediaTime::new(96_000, base))
            .with_musical(MusicalTimeInfo {
                position_ticks: 1_920,
                bar_start_tick: 960,
                bpm_milli: 120_500,
                ppqn: 960,
                sig_num: 3,
                sig_den: 4,
            });
        let context =
            map_process_context(Some(transport), 48_000).expect("Some transport maps to a context");
        let expected = (ProcessContext_::StatesAndFlags_::kPlaying
            | ProcessContext_::StatesAndFlags_::kProjectTimeMusicValid
            | ProcessContext_::StatesAndFlags_::kTempoValid
            | ProcessContext_::StatesAndFlags_::kTimeSigValid
            | ProcessContext_::StatesAndFlags_::kBarPositionValid) as u32;
        assert_eq!(context.state, expected);
        assert!((context.sampleRate - 48_000.0).abs() < f64::EPSILON);
        assert_eq!(context.projectTimeSamples, 96_000);
        // `continousTimeSamples` stays zero — `position()` is the
        // playhead, not engine-continuous time, so `kContTimeValid`
        // is never raised (see `map_process_context`).
        assert_eq!(context.continousTimeSamples, 0);
        assert!((context.projectTimeMusic - 2.0).abs() < f64::EPSILON);
        assert!((context.barPositionMusic - 1.0).abs() < f64::EPSILON);
        assert!((context.tempo - 120.5).abs() < f64::EPSILON);
        assert_eq!(context.timeSigNumerator, 3);
        assert_eq!(context.timeSigDenominator, 4);
        assert!(map_process_context(None, 48_000).is_none());
    }

    #[test]
    fn process_with_transport_snapshot_is_defined() {
        // `Some(transport)` through `process` — the non-null
        // `processContext` path round-trips the fixture's own mapping
        // (`component.rs::transport`) without rejection; output stays
        // the defined gain-1.0 passthrough.
        let mut plugin = prepared_plugin();
        let base = TimeBase::sample_rate(48_000).unwrap();
        let transport = TransportInfo::new(TransportStatus::Playing, MediaTime::new(1_024, base));
        let (result, [left, right]) =
            drive_block(&mut *plugin, EventList::empty(FRAMES_U32), Some(transport));
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(left, vec![0.5_f32; FRAMES]);
        assert_eq!(right, vec![-0.25_f32; FRAMES]);
    }

    /// Issue #253 (b): the fixture controller's `IMidiMapping` routes
    /// volume CC 7 through `inputParameterChanges` — the canonical
    /// `ControlChange` scales the block instead of reaching the
    /// plugin as an ordinary MIDI event.
    #[test]
    #[expect(clippy::cast_possible_truncation)]
    fn imidi_mapping_routes_volume_cc_to_gain_param() {
        let mut plugin = prepared_plugin();
        let cc = MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::ControlChange {
                controller: MidiController::new(7).unwrap(),
                value: MidiValue::from_u7(63),
            },
        };
        let events = [Event::new(0, EventKind::Midi(cc))];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, right]) = drive_block(&mut *plugin, events, None);
        assert_eq!(result.status, ProcessStatus::Continue);
        // `IMidiMapping` semantics: CC 7 → normalized `63/127`.
        let gain = (63.0_f64 / 127.0) as f32;
        assert_eq!(left, vec![0.5_f32 * gain; FRAMES]);
        assert_eq!(right, vec![-0.25_f32 * gain; FRAMES]);
        // The routed event produced the fixture's applied-value
        // report — proof it arrived as a parameter, not a MIDI event.
        let mut reports = Vec::new();
        plugin.drain_output_reports(&mut reports);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].frame_offset(), 0);
        let EventKind::ParamValue(param) = reports[0].kind() else {
            panic!("routed CC must surface as a ParamValue report");
        };
        assert_eq!(param.param_id, ParamId(0));
    }

    /// An unmapped controller keeps the ordinary MIDI path: the
    /// fixture ignores it (passthrough audio) and nothing reports.
    #[test]
    fn unmapped_cc_passes_through_without_param_routing() {
        let mut plugin = prepared_plugin();
        let cc = MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::ControlChange {
                controller: MidiController::new(20).unwrap(),
                value: MidiValue::from_u7(100),
            },
        };
        let events = [Event::new(0, EventKind::Midi(cc))];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, right]) = drive_block(&mut *plugin, events, None);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(left, vec![0.5_f32; FRAMES]);
        assert_eq!(right, vec![-0.25_f32; FRAMES]);
        let mut reports = Vec::new();
        plugin.drain_output_reports(&mut reports);
        assert_eq!(reports, Vec::<Event>::new());
    }

    /// Issue #253 (c): the fixture's plugin→host reports round-trip —
    /// applied `ParamValue`s echo through `outputParameterChanges`,
    /// `NoteOn`s through `outputEvents`, and the host collects both
    /// into canonical `Event`s via `drain_output_reports`.
    #[test]
    fn plugin_output_reports_drain_as_canonical_events() {
        let mut plugin = prepared_plugin();
        let note_on = MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        };
        let events = [
            Event::new(
                0,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(0),
                    value: ParamValue::new(0.5).unwrap(),
                }),
            ),
            Event::new(1, EventKind::Midi(note_on)),
        ];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, _right]) = drive_block(&mut *plugin, events, None);
        assert_eq!(result.status, ProcessStatus::Continue);
        // The gain parameter applied — `0.5 * 0.5` / `-0.25 * 0.5`.
        assert_eq!(left, vec![0.25_f32; FRAMES]);

        let mut reports = Vec::new();
        plugin.drain_output_reports(&mut reports);
        assert_eq!(reports.len(), 2);
        // Sorted by frame offset: `ParamValue`@0 then `Midi`@1.
        assert_eq!(reports[0].frame_offset(), 0);
        let EventKind::ParamValue(param) = reports[0].kind() else {
            panic!("first report must be the ParamValue echo");
        };
        assert_eq!(param.param_id, ParamId(0));
        assert_eq!(param.value.get(), 0.5);
        assert_eq!(reports[1].frame_offset(), 1);
        let EventKind::Midi(midi) = reports[1].kind() else {
            panic!("second report must be the Midi echo");
        };
        assert!(matches!(midi.kind, MidiKind::NoteOn { .. }));
    }

    // -----------------------------------------------------------------
    // Stateful COM doubles — the in-process oracle for the state
    // bridge (design `issue-630-adapter-state-bridge`). These emulate
    // a *stateful* plugin's verbs; they are not the real-ABI oracle —
    // that arrives with the `test_stateful` fixture slice.
    // -----------------------------------------------------------------

    /// Ordered call log shared by the component/controller doubles —
    /// pins the VST3 state-verb sequence across both objects.
    type CallLog = Rc<RefCell<Vec<&'static str>>>;

    /// Drains an `IBStream` into a `Vec` — the plugin-side read half.
    #[expect(unsafe_code)]
    unsafe fn stream_read_all(state: *mut IBStream) -> Result<Vec<u8>, tresult> {
        let Some(stream) = (unsafe { ComRef::from_raw(state) }) else {
            return Err(kInvalidArgument);
        };
        let mut out = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            let mut got = 0i32;
            // SAFETY: `stream` is a valid `IBStream` for the call's
            // duration; `buf` is a live 256-byte destination.
            let result = unsafe {
                stream.read(
                    buf.as_mut_ptr().cast(),
                    i32::try_from(buf.len()).unwrap_or(i32::MAX),
                    &raw mut got,
                )
            };
            if result != kResultOk {
                return Err(result);
            }
            if got <= 0 {
                return Ok(out);
            }
            out.extend_from_slice(&buf[..usize::try_from(got).unwrap_or(0)]);
        }
    }

    /// Writes `bytes` into an `IBStream` — the plugin-side write half.
    #[expect(unsafe_code)]
    unsafe fn stream_write_all(state: *mut IBStream, bytes: &[u8]) -> tresult {
        let Some(stream) = (unsafe { ComRef::from_raw(state) }) else {
            return kInvalidArgument;
        };
        let mut written = 0i32;
        // SAFETY: `stream` is a valid `IBStream`; `bytes` is a live
        // readable slice of the reported length.
        unsafe {
            stream.write(
                bytes.as_ptr() as *mut c_void,
                i32::try_from(bytes.len()).unwrap_or(i32::MAX),
                &raw mut written,
            )
        }
    }

    /// Test-only stateful `IComponent` — `setState` drains the stream
    /// into `stored`, `getState` writes `stored` back, the way a real
    /// plugin's state verbs behave. `state_result` overrides both
    /// verbs' return for the decline/failure arms.
    struct TestStatefulComponent {
        stored: RefCell<Vec<u8>>,
        state_result: Cell<tresult>,
        log: CallLog,
    }

    impl TestStatefulComponent {
        fn new(log: CallLog) -> Self {
            Self {
                stored: RefCell::new(Vec::new()),
                state_result: Cell::new(kResultOk),
                log,
            }
        }
    }

    impl Class for TestStatefulComponent {
        type Interfaces = (IComponent,);
    }

    impl IPluginBaseTrait for TestStatefulComponent {
        #[expect(unsafe_code)]
        unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn terminate(&self) -> tresult {
            kResultOk
        }
    }

    #[expect(unsafe_code)]
    impl IComponentTrait for TestStatefulComponent {
        unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
            if class_id.is_null() {
                return kInvalidArgument;
            }
            // SAFETY: null-checked above; a CID distinct from any
            // component CID — the distributed-plugin shape.
            unsafe { class_id.write([0x5A; 16]) };
            kResultOk
        }

        unsafe fn setIoMode(&self, _mode: i32) -> tresult {
            kResultOk
        }

        unsafe fn getBusCount(&self, _media_type: i32, _dir: i32) -> i32 {
            0
        }

        unsafe fn getBusInfo(
            &self,
            _media_type: i32,
            _dir: i32,
            _index: i32,
            _bus: *mut BusInfo,
        ) -> tresult {
            kResultFalse
        }

        unsafe fn getRoutingInfo(
            &self,
            _in_info: *mut RoutingInfo,
            _out_info: *mut RoutingInfo,
        ) -> tresult {
            kResultFalse
        }

        unsafe fn activateBus(
            &self,
            _media_type: i32,
            _dir: i32,
            _index: i32,
            _state: u8,
        ) -> tresult {
            kResultOk
        }

        unsafe fn setActive(&self, _state: u8) -> tresult {
            kResultOk
        }

        unsafe fn setState(&self, state: *mut IBStream) -> tresult {
            self.log.borrow_mut().push("component.setState");
            let override_result = self.state_result.get();
            if override_result != kResultOk {
                return override_result;
            }
            match unsafe { stream_read_all(state) } {
                Ok(bytes) => {
                    *self.stored.borrow_mut() = bytes;
                    kResultOk
                }
                Err(result) => result,
            }
        }

        unsafe fn getState(&self, state: *mut IBStream) -> tresult {
            self.log.borrow_mut().push("component.getState");
            let override_result = self.state_result.get();
            if override_result != kResultOk {
                return override_result;
            }
            unsafe { stream_write_all(state, &self.stored.borrow()) }
        }
    }

    /// Test-only stateful `IEditController` — mirrors the component
    /// double: `setComponentState` stores the normalized component
    /// payload separately from `controller_state` so tests can prove
    /// the hand-off content, and `log` records the call order.
    struct TestStatefulController {
        component_state: RefCell<Vec<u8>>,
        controller_state: RefCell<Vec<u8>>,
        state_result: Cell<tresult>,
        log: CallLog,
    }

    impl TestStatefulController {
        fn new(log: CallLog) -> Self {
            Self {
                component_state: RefCell::new(Vec::new()),
                controller_state: RefCell::new(Vec::new()),
                state_result: Cell::new(kResultOk),
                log,
            }
        }
    }

    impl Class for TestStatefulController {
        type Interfaces = (IEditController,);
    }

    impl IPluginBaseTrait for TestStatefulController {
        #[expect(unsafe_code)]
        unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn terminate(&self) -> tresult {
            kResultOk
        }
    }

    #[expect(unsafe_code)]
    impl IEditControllerTrait for TestStatefulController {
        unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
            self.log.borrow_mut().push("controller.setComponentState");
            let override_result = self.state_result.get();
            if override_result != kResultOk {
                return override_result;
            }
            match unsafe { stream_read_all(state) } {
                Ok(bytes) => {
                    *self.component_state.borrow_mut() = bytes;
                    kResultOk
                }
                Err(result) => result,
            }
        }

        unsafe fn setState(&self, state: *mut IBStream) -> tresult {
            self.log.borrow_mut().push("controller.setState");
            let override_result = self.state_result.get();
            if override_result != kResultOk {
                return override_result;
            }
            match unsafe { stream_read_all(state) } {
                Ok(bytes) => {
                    *self.controller_state.borrow_mut() = bytes;
                    kResultOk
                }
                Err(result) => result,
            }
        }

        unsafe fn getState(&self, state: *mut IBStream) -> tresult {
            self.log.borrow_mut().push("controller.getState");
            let override_result = self.state_result.get();
            if override_result != kResultOk {
                return override_result;
            }
            unsafe { stream_write_all(state, &self.controller_state.borrow()) }
        }

        unsafe fn getParameterCount(&self) -> i32 {
            0
        }

        unsafe fn getParameterInfo(&self, _param_index: i32, _info: *mut ParameterInfo) -> tresult {
            kResultFalse
        }

        unsafe fn getParamStringByValue(
            &self,
            _id: ParamID,
            _value_normalized: f64,
            _string: *mut String128,
        ) -> tresult {
            kResultFalse
        }

        unsafe fn getParamValueByString(
            &self,
            _id: ParamID,
            _string: *mut TChar,
            _value_normalized: *mut f64,
        ) -> tresult {
            kResultFalse
        }

        unsafe fn normalizedParamToPlain(&self, _id: ParamID, value_normalized: f64) -> f64 {
            value_normalized
        }

        unsafe fn plainParamToNormalized(&self, _id: ParamID, plain_value: f64) -> f64 {
            plain_value
        }

        unsafe fn getParamNormalized(&self, _id: ParamID) -> f64 {
            0.0
        }

        unsafe fn setParamNormalized(&self, _id: ParamID, _value: f64) -> tresult {
            kResultOk
        }

        unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
            kResultOk
        }

        unsafe fn createView(&self, _name: *const char8) -> *mut IPlugView {
            ptr::null_mut()
        }
    }

    /// Builds the distributed-plugin pair: a stateful component and
    /// controller sharing one call log.
    fn stateful_pair() -> (
        ComWrapper<TestStatefulComponent>,
        ComPtr<IComponent>,
        ComWrapper<TestStatefulController>,
        ComPtr<IEditController>,
        CallLog,
    ) {
        let log: CallLog = Rc::new(RefCell::new(Vec::new()));
        let comp_wrapper = ComWrapper::new(TestStatefulComponent::new(log.clone()));
        let comp = comp_wrapper
            .to_com_ptr::<IComponent>()
            .expect("component exposes IComponent");
        let ctrl_wrapper = ComWrapper::new(TestStatefulController::new(log.clone()));
        let ctrl = ctrl_wrapper
            .to_com_ptr::<IEditController>()
            .expect("controller exposes IEditController");
        (comp_wrapper, comp, ctrl_wrapper, ctrl, log)
    }

    #[test]
    #[expect(unsafe_code)]
    fn bridge_stream_read_write_seek_tell_roundtrip() {
        let wrapper = ComWrapper::new(BridgeStream::new());
        let stream = wrapper
            .to_com_ptr::<IBStream>()
            .expect("stream exposes IBStream");
        let bytes = b"hello";
        // SAFETY: `stream` is a valid `IBStream` for the call's
        // duration; `bytes` is a live readable slice.
        unsafe {
            let mut written = 0i32;
            assert_eq!(
                stream.write(bytes.as_ptr() as *mut c_void, 5, &raw mut written),
                kResultOk
            );
            assert_eq!(written, 5);
            let mut pos = 0i64;
            assert_eq!(stream.tell(&raw mut pos), kResultOk);
            assert_eq!(pos, 5);
            // Rewind and read the payload back.
            let mut result = 0i64;
            assert_eq!(
                stream.seek(0, sdk_enum_i32(kIBSeekSet), &raw mut result),
                kResultOk
            );
            assert_eq!(result, 0);
            let mut buf = [0u8; 16];
            let mut got = 0i32;
            assert_eq!(
                stream.read(buf.as_mut_ptr().cast(), 16, &raw mut got),
                kResultOk
            );
            // Honest partial read — only the 5 written bytes exist.
            assert_eq!(got, 5);
            assert_eq!(&buf[..5], b"hello");
            // EOF: the next read reports 0 bytes, still `kResultOk`.
            let mut got = -1i32;
            assert_eq!(
                stream.read(buf.as_mut_ptr().cast(), 16, &raw mut got),
                kResultOk
            );
            assert_eq!(got, 0);
        }
        assert_eq!(wrapper.take_bytes(), b"hello");
    }

    #[test]
    #[expect(unsafe_code)]
    fn bridge_stream_rejects_invalid_arguments() {
        let wrapper = ComWrapper::new(BridgeStream::new());
        let stream = wrapper
            .to_com_ptr::<IBStream>()
            .expect("stream exposes IBStream");
        let mut buf = [0u8; 8];
        let mut out = 0i32;
        // SAFETY: all pointers are either valid or deliberately null
        // to exercise the defined refusals.
        unsafe {
            // Null buffer with a positive count.
            assert_eq!(
                stream.read(ptr::null_mut(), 4, &raw mut out),
                kInvalidArgument
            );
            // Negative byte count.
            assert_eq!(
                stream.read(buf.as_mut_ptr().cast(), -1, &raw mut out),
                kInvalidArgument
            );
            assert_eq!(
                stream.write(ptr::null_mut(), 4, &raw mut out),
                kInvalidArgument
            );
            assert_eq!(
                stream.write(buf.as_ptr() as *mut c_void, -1, &raw mut out),
                kInvalidArgument
            );
            // Unknown seek mode.
            assert_eq!(stream.seek(0, 99, ptr::null_mut()), kInvalidArgument);
            // Negative target position.
            assert_eq!(
                stream.seek(-1, sdk_enum_i32(kIBSeekSet), ptr::null_mut()),
                kResultFalse
            );
            // Overflowing target position — needs a nonzero base, so
            // write a byte first (`kIBSeekEnd` bases on `data.len()`).
            let mut written = 0i32;
            assert_eq!(
                stream.write(b"x".as_ptr() as *mut c_void, 1, &raw mut written),
                kResultOk
            );
            assert_eq!(
                stream.seek(i64::MAX, sdk_enum_i32(kIBSeekEnd), ptr::null_mut()),
                kResultFalse
            );
            // Null out-pointers are tolerated — `tell`/`seek` still
            // perform the operation.
            assert_eq!(stream.tell(ptr::null_mut()), kResultOk);
            assert_eq!(
                stream.seek(0, sdk_enum_i32(kIBSeekSet), ptr::null_mut()),
                kResultOk
            );
        }
    }

    #[test]
    #[expect(unsafe_code)]
    fn bridge_stream_sparse_write_and_bounded_reads() {
        let wrapper = ComWrapper::new(BridgeStream::new());
        let stream = wrapper
            .to_com_ptr::<IBStream>()
            .expect("stream exposes IBStream");
        // SAFETY: `stream` is a valid `IBStream` for the call's
        // duration; buffers are live for their reported lengths.
        unsafe {
            let mut written = 0i32;
            assert_eq!(
                stream.write(b"hi".as_ptr() as *mut c_void, 2, &raw mut written),
                kResultOk
            );
            // Forward seek past the end, then write — the gap is
            // zero-filled, never fabricated content.
            assert_eq!(
                stream.seek(4, sdk_enum_i32(kIBSeekSet), ptr::null_mut()),
                kResultOk
            );
            assert_eq!(
                stream.write(b"yo".as_ptr() as *mut c_void, 2, &raw mut written),
                kResultOk
            );
            assert_eq!(written, 2);
            // Bounded read: ask for 3 of the 6 bytes.
            assert_eq!(
                stream.seek(0, sdk_enum_i32(kIBSeekSet), ptr::null_mut()),
                kResultOk
            );
            let mut buf = [0xFFu8; 8];
            let mut got = 0i32;
            assert_eq!(
                stream.read(buf.as_mut_ptr().cast(), 3, &raw mut got),
                kResultOk
            );
            assert_eq!(got, 3);
            assert_eq!(&buf[..3], b"hi\0");
            // Seek beyond the end then read — honest zero-byte EOF.
            assert_eq!(
                stream.seek(99, sdk_enum_i32(kIBSeekSet), ptr::null_mut()),
                kResultOk
            );
            let mut got = -1i32;
            assert_eq!(
                stream.read(buf.as_mut_ptr().cast(), 8, &raw mut got),
                kResultOk
            );
            assert_eq!(got, 0);
        }
        assert_eq!(wrapper.take_bytes(), b"hi\0\0yo");
    }

    #[test]
    fn frame_state_roundtrip_and_malformed_rejected() {
        let blob = frame_state(b"component-payload", b"controller-payload");
        let (comp, ctrl) = unframe_state(&blob).expect("well-formed frame parses");
        assert_eq!(comp, b"component-payload");
        assert_eq!(ctrl, b"controller-payload");
        // Empty payloads still produce a parseable 8-byte frame.
        let empty = frame_state(b"", b"");
        let (comp, ctrl) = unframe_state(&empty).expect("empty frame parses");
        assert!(comp.is_empty() && ctrl.is_empty());
        // Corrupt shapes all decline.
        let mut trailing = blob.clone();
        trailing.push(0);
        assert!(unframe_state(&trailing).is_none());
        let mut oversized = Vec::new();
        oversized.extend_from_slice(&u32::MAX.to_le_bytes());
        oversized.extend_from_slice(b"x");
        assert!(unframe_state(&oversized).is_none());
        assert!(unframe_state(&blob[..blob.len() - 1]).is_none());
    }

    #[test]
    fn state_roundtrip_distributed_order_and_payloads() {
        let (comp_wrapper, comp, ctrl_wrapper, ctrl, log) = stateful_pair();
        *comp_wrapper.stored.borrow_mut() = b"component-state".to_vec();
        *ctrl_wrapper.controller_state.borrow_mut() = b"controller-state".to_vec();

        let (comp_bytes, ctrl_bytes) =
            save_component_state(&comp, Some(&ctrl), false).expect("save succeeds");
        assert_eq!(comp_bytes, b"component-state");
        assert_eq!(ctrl_bytes, b"controller-state");
        assert_eq!(*log.borrow(), ["component.getState", "controller.getState"]);

        // Dirty the live state, then restore the framed blob.
        *comp_wrapper.stored.borrow_mut() = b"dirty".to_vec();
        ctrl_wrapper.component_state.borrow_mut().clear();
        ctrl_wrapper.controller_state.borrow_mut().clear();
        log.borrow_mut().clear();

        let blob = frame_state(&comp_bytes, &ctrl_bytes);
        restore_component_state(&comp, Some(&ctrl), false, &blob).expect("restore succeeds");

        assert_eq!(*comp_wrapper.stored.borrow(), b"component-state");
        assert_eq!(*ctrl_wrapper.controller_state.borrow(), b"controller-state");
        // `setComponentState` receives the *re-serialized* component
        // state — the VST3 hand-off, not the raw input blob.
        assert_eq!(*ctrl_wrapper.component_state.borrow(), b"component-state");
        assert_eq!(
            *log.borrow(),
            [
                "component.setState",
                "component.getState",
                "controller.setComponentState",
                "controller.setState",
            ]
        );
    }

    #[test]
    fn state_roundtrip_single_component_skips_controller() {
        // `single_component == true` — the component *is* the
        // controller: no controller verbs run and the framed ctrl
        // payload is empty (a second `getState` would re-serialize
        // the same object).
        let (comp_wrapper, comp, _ctrl_wrapper, ctrl, log) = stateful_pair();
        *comp_wrapper.stored.borrow_mut() = b"solo-state".to_vec();

        let (comp_bytes, ctrl_bytes) =
            save_component_state(&comp, Some(&ctrl), true).expect("save succeeds");
        assert_eq!(comp_bytes, b"solo-state");
        assert_eq!(ctrl_bytes.len(), 0);
        assert_eq!(*log.borrow(), ["component.getState"]);

        log.borrow_mut().clear();
        let blob = frame_state(&comp_bytes, &ctrl_bytes);
        restore_component_state(&comp, Some(&ctrl), true, &blob).expect("restore succeeds");
        assert_eq!(*log.borrow(), ["component.setState", "component.getState"]);
        assert_eq!(*comp_wrapper.stored.borrow(), b"solo-state");
    }

    #[test]
    fn state_restore_rejects_corrupt_frame_before_plugin_calls() {
        let (_comp_wrapper, comp, _ctrl_wrapper, ctrl, log) = stateful_pair();
        // Trailing byte after a well-formed frame.
        let mut blob = frame_state(b"c", b"t");
        blob.push(0);
        assert_eq!(
            restore_component_state(&comp, Some(&ctrl), false, &blob),
            Err(HostError::PluginFailed)
        );
        // `comp_len` larger than the payload.
        let mut blob = Vec::new();
        blob.extend_from_slice(&100u32.to_le_bytes());
        blob.extend_from_slice(b"short");
        assert_eq!(
            restore_component_state(&comp, Some(&ctrl), false, &blob),
            Err(HostError::PluginFailed)
        );
        // No state verb ran — rejection happens at the boundary.
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn state_restore_foreign_blob_delegates_to_plugin() {
        // Shorter than `FRAME_HEADER_BYTES` — cannot be this
        // adapter's serialization, so the raw blob becomes the
        // component payload and the plugin owns validation (the L1
        // e2e's `[0xAA, 0xBB]` case).
        let (comp_wrapper, comp, ctrl_wrapper, ctrl, log) = stateful_pair();
        let blob = [0xAA, 0xBB];
        restore_component_state(&comp, Some(&ctrl), false, &blob).expect("restore succeeds");
        assert_eq!(*comp_wrapper.stored.borrow(), blob.to_vec());
        // The normalized component state reaches `setComponentState`;
        // the empty ctrl payload reaches `setState`.
        assert_eq!(*ctrl_wrapper.component_state.borrow(), blob.to_vec());
        assert!(ctrl_wrapper.controller_state.borrow().is_empty());
        assert_eq!(
            *log.borrow(),
            [
                "component.setState",
                "component.getState",
                "controller.setComponentState",
                "controller.setState",
            ]
        );
    }

    #[test]
    fn state_knotimplemented_maps_to_unsupported() {
        let (comp_wrapper, comp, ctrl_wrapper, ctrl, _log) = stateful_pair();
        comp_wrapper.state_result.set(kNotImplemented);
        ctrl_wrapper.state_result.set(kNotImplemented);
        // Save declines at the first state verb.
        assert_eq!(
            save_component_state(&comp, Some(&ctrl), false),
            Err(HostError::Unsupported)
        );
        // Restore declines at `component.setState`.
        let blob = frame_state(b"x", b"y");
        assert_eq!(
            restore_component_state(&comp, Some(&ctrl), false, &blob),
            Err(HostError::Unsupported)
        );
        // Any other failure is a plugin failure, not a capability
        // answer.
        comp_wrapper.state_result.set(kResultFalse);
        assert_eq!(
            save_component_state(&comp, Some(&ctrl), false),
            Err(HostError::PluginFailed)
        );
    }

    // -----------------------------------------------------------------
    // `BridgePlugFrame` request-route tests (Issue #873) — the frame
    // is the unit under test; `TestPlugView` is the COM-double view
    // that receives the `onSize` roundtrip.
    // -----------------------------------------------------------------

    /// Test-only `IPlugView` — records each `onSize` delivery's
    /// `(width, height)`; every other verb is a defined stub. The
    /// `resizeView` verdict logic is the unit under test, not the
    /// view.
    struct TestPlugView {
        /// `(right-left, bottom-top)` per `onSize`, in call order.
        on_size_log: RefCell<Vec<(i32, i32)>>,
    }

    impl Class for TestPlugView {
        type Interfaces = (IPlugView,);
    }

    impl IPlugViewTrait for TestPlugView {
        #[expect(unsafe_code)]
        unsafe fn isPlatformTypeSupported(&self, _type: *const char8) -> tresult {
            kResultTrue
        }

        #[expect(unsafe_code)]
        unsafe fn attached(&self, _parent: *mut c_void, _type: *const char8) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn removed(&self) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn onWheel(&self, _distance: f32) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn onKeyDown(&self, _key: u16, _key_code: i16, _modifiers: i16) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn onKeyUp(&self, _key: u16, _key_code: i16, _modifiers: i16) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn getSize(&self, _size: *mut ViewRect) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn onSize(&self, new_size: *mut ViewRect) -> tresult {
            // SAFETY: the frame passes a live `ViewRect` it owns for
            // the call's duration.
            let rect = unsafe { *new_size };
            self.on_size_log
                .borrow_mut()
                .push((rect.right - rect.left, rect.bottom - rect.top));
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn onFocus(&self, _state: TBool) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn setFrame(&self, _frame: *mut IPlugFrame) -> tresult {
            kResultOk
        }

        #[expect(unsafe_code)]
        unsafe fn canResize(&self) -> tresult {
            kResultTrue
        }

        #[expect(unsafe_code)]
        unsafe fn checkSizeConstraint(&self, _rect: *mut ViewRect) -> tresult {
            kResultOk
        }
    }

    /// The shared request-sink cell — `Vst3Plugin`'s side in
    /// production, held directly by the test.
    type SinkCell = Rc<RefCell<Option<Box<GuiRequestSink>>>>;

    /// A `BridgePlugFrame` COM pair plus a view double sharing the
    /// request-sink cell — the same shape `open_gui` wires.
    struct PlugFrameFixture {
        /// Keeps the frame's `ComWrapper` (vtable holder) alive.
        _frame_wrapper: ComWrapper<BridgePlugFrame>,
        frame: ComPtr<IPlugFrame>,
        /// Keeps the view's `ComWrapper` alive.
        view_wrapper: ComWrapper<TestPlugView>,
        view: ComPtr<IPlugView>,
        cell: SinkCell,
    }

    fn plug_frame_pair() -> PlugFrameFixture {
        let cell: SinkCell = Rc::new(RefCell::new(None));
        let frame_wrapper = ComWrapper::new(BridgePlugFrame {
            request_sink: Rc::clone(&cell),
        });
        let frame = frame_wrapper
            .to_com_ptr::<IPlugFrame>()
            .expect("frame exposes IPlugFrame");
        let view_wrapper = ComWrapper::new(TestPlugView {
            on_size_log: RefCell::new(Vec::new()),
        });
        let view = view_wrapper
            .to_com_ptr::<IPlugView>()
            .expect("view exposes IPlugView");
        PlugFrameFixture {
            _frame_wrapper: frame_wrapper,
            frame,
            view_wrapper,
            view,
            cell,
        }
    }

    /// A 640×480 `ViewRect` — the plugin's requested size.
    fn request_rect() -> ViewRect {
        ViewRect {
            left: 0,
            top: 0,
            right: 640,
            bottom: 480,
        }
    }

    #[test]
    #[expect(unsafe_code)]
    fn plug_frame_resize_accept_runs_onsize_roundtrip() {
        let fixture = plug_frame_pair();
        // Scripted route — accepts with the corrected 320×240.
        *fixture.cell.borrow_mut() = Some(Box::new(|request| {
            assert_eq!(
                request,
                GuiRequest::Resize {
                    size: GuiSize {
                        width: 640.0,
                        height: 480.0,
                    }
                }
            );
            GuiRequestVerdict::Accepted {
                granted: Some(GuiSize {
                    width: 320.0,
                    height: 240.0,
                }),
            }
        }));
        let mut rect = request_rect();
        // SAFETY: `view`/`frame` are live COM objects for the call's
        // duration; `rect` is a live `ViewRect`.
        let result = unsafe {
            fixture
                .frame
                .resizeView(fixture.view.as_ptr(), &raw mut rect)
        };
        assert_eq!(result, kResultOk);
        // The `onSize` roundtrip delivered the granted size, not the
        // requested one.
        assert_eq!(*fixture.view_wrapper.on_size_log.borrow(), vec![(320, 240)]);
    }

    #[test]
    #[expect(unsafe_code)]
    fn plug_frame_resize_accept_verbatim_onsize() {
        let fixture = plug_frame_pair();
        // `granted: None` — the requested size applies verbatim.
        *fixture.cell.borrow_mut() =
            Some(Box::new(|_| GuiRequestVerdict::Accepted { granted: None }));
        let mut rect = request_rect();
        // SAFETY: live COM objects + a live `ViewRect` for the call.
        let result = unsafe {
            fixture
                .frame
                .resizeView(fixture.view.as_ptr(), &raw mut rect)
        };
        assert_eq!(result, kResultOk);
        assert_eq!(*fixture.view_wrapper.on_size_log.borrow(), vec![(640, 480)]);
    }

    #[test]
    #[expect(unsafe_code)]
    fn plug_frame_resize_refusal_and_unrouted_stay_false() {
        let fixture = plug_frame_pair();
        // Unrouted — no sink installed: the defined refusal, no
        // `onSize` delivery.
        let mut rect = request_rect();
        // SAFETY: live COM objects + a live `ViewRect` for the call.
        assert_eq!(
            unsafe {
                fixture
                    .frame
                    .resizeView(fixture.view.as_ptr(), &raw mut rect)
            },
            kResultFalse
        );
        // Routed refusal — the arbiter declines: still `kResultFalse`,
        // still no `onSize`.
        *fixture.cell.borrow_mut() = Some(Box::new(|_| GuiRequestVerdict::Refused {
            reason: GuiRequestRefusal::Rejected,
        }));
        // SAFETY: live COM objects + a live `ViewRect` for the call.
        assert_eq!(
            unsafe {
                fixture
                    .frame
                    .resizeView(fixture.view.as_ptr(), &raw mut rect)
            },
            kResultFalse
        );
        // Re-entrant — the cell is borrowed elsewhere when the
        // request arrives: the defined refusal, never a `RefCell`
        // panic across the ABI.
        let guard = fixture.cell.borrow_mut();
        // SAFETY: live COM objects + a live `ViewRect` for the call.
        assert_eq!(
            unsafe {
                fixture
                    .frame
                    .resizeView(fixture.view.as_ptr(), &raw mut rect)
            },
            kResultFalse
        );
        drop(guard);
        assert_eq!(*fixture.view_wrapper.on_size_log.borrow(), Vec::new());
    }

    #[test]
    #[expect(unsafe_code)]
    fn plug_frame_resize_rejects_null_arguments() {
        let fixture = plug_frame_pair();
        let mut rect = request_rect();
        // SAFETY: null arguments exercise the defined argument
        // refusals; live pointers carry the valid call.
        unsafe {
            assert_eq!(
                fixture.frame.resizeView(ptr::null_mut(), &raw mut rect),
                kInvalidArgument
            );
            assert_eq!(
                fixture
                    .frame
                    .resizeView(fixture.view.as_ptr(), ptr::null_mut()),
                kInvalidArgument
            );
        }
    }

    /// `IComponentHandler`/`IComponentHandler2` dirty surfaces enqueue
    /// neutral notices (Issue #891): `performEdit` → `ParamEdited` +
    /// the latched flag edge, `setDirty(bool)` → the flag edge,
    /// `restartComponent(kParamValuesChanged)` → `StateDirty` while
    /// every other restart bit stays the honest refusal.
    #[test]
    #[expect(unsafe_code)]
    fn component_handler_dirty_surfaces_enqueue_notices() {
        let notices = Rc::new(RefCell::new(DirtyNoticeQueue::new()));
        let wrapper = ComWrapper::new(BridgeComponentHandler::new(Rc::clone(&notices)));
        let handler = wrapper.to_com_ptr::<IComponentHandler>().unwrap();
        let handler2 = handler.cast::<IComponentHandler2>().unwrap();

        // `performEdit` — finite value rides the `ParamEdited`
        // payload; the same enqueue latches the flag edge after it.
        // SAFETY: `handler`/`handler2` are live for the call.
        assert_eq!(unsafe { handler.performEdit(7, 0.625) }, kResultOk);
        // `setDirty(false)` — the plugin declares re-synced state.
        assert_eq!(unsafe { handler2.setDirty(0) }, kResultOk);
        // `setDirty(true)` — dirty again.
        assert_eq!(unsafe { handler2.setDirty(1) }, kResultOk);
        // `kParamValuesChanged` alone → dirty + `kResultOk`.
        assert_eq!(
            unsafe { handler.restartComponent(RestartFlags_::kParamValuesChanged) },
            kResultOk
        );
        // Gesture edges stay acknowledged no-ops.
        assert_eq!(unsafe { handler.beginEdit(7) }, kResultOk);
        assert_eq!(unsafe { handler.endEdit(7) }, kResultOk);
        // Every other restart bit is the honest refusal.
        assert_eq!(
            unsafe { handler.restartComponent(RestartFlags_::kReloadComponent) },
            kNotImplemented
        );
        assert_eq!(
            unsafe {
                handler.restartComponent(
                    RestartFlags_::kParamValuesChanged | RestartFlags_::kLatencyChanged,
                )
            },
            kNotImplemented
        );
        // `IComponentHandler2` siblings the bridge does not serve —
        // honest refusals, never fabricated acks.
        assert_eq!(
            unsafe { handler2.requestOpenEditor(ptr::null()) },
            kNotImplemented
        );
        assert_eq!(unsafe { handler2.startGroupEdit() }, kNotImplemented);
        assert_eq!(unsafe { handler2.finishGroupEdit() }, kNotImplemented);

        let mut out = Vec::new();
        notices.borrow_mut().drain(&mut out);
        assert_eq!(
            out,
            vec![
                DirtyNotice::ParamEdited {
                    param_id: ParamId(7),
                    value: ParamValue::new(0.625).unwrap(),
                },
                DirtyNotice::StateDirty,
            ]
        );

        // Non-finite `performEdit` degrades to the bare flag — the
        // divergence is still reported, never silently dropped.
        assert_eq!(unsafe { handler.performEdit(7, f64::NAN) }, kResultOk);
        let mut out = Vec::new();
        notices.borrow_mut().drain(&mut out);
        assert_eq!(out, vec![DirtyNotice::StateDirty]);

        // `restartComponent` with unhandled bits enqueued nothing —
        // the refusal did not dirty the queue.
        let mut out = Vec::new();
        notices.borrow_mut().drain(&mut out);
        assert_eq!(out, []);
    }
}
