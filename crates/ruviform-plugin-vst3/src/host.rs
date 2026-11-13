//! Host-side VST3 bridge (`feature = "host"`): dlopens a real VST3
//! module in-process via `libloading`, scans its class table through
//! the raw `vst3`-crate COM bindings and maps descriptors, parameters
//! and the component/controller lifecycle onto the neutral
//! [`PluginFactory`]/[`Plugin`] contracts for the L1 out-of-process
//! runner (Issue #114 — `docs/tasks/vst3-host-bridge.md`).
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
//! - `BridgeComponentHandler` — `IComponentHandler`; injected via
//!   `setComponentHandler` and *acknowledge-only*: the fixture's
//!   handler is non-retaining (`controller.rs`), and routing
//!   `performEdit`/`restartComponent` back into the component is a
//!   defined boundary — the runner's
//!   only param path is the `process` event list. `beginEdit`/
//!   `performEdit`/`endEdit` still answer `kResultOk` (a refused edit
//!   could wedge the plugin's UI thread); `restartComponent` answers
//!   `kNotImplemented` — `kResultOk` would promise the flag-requested
//!   resend (param values / component state) the bridge never
//!   performs.
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
//! - `save_state`/`restore_state` keep the [`Plugin`] defaults
//!   (`Err(HostError::Unsupported)`) — the bridge never calls
//!   `setState`/`getState`/`setComponentState`.
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
use std::sync::Arc;

use libloading::Library;
use ruviform_media_core::{Rounding, TimeBase, TransportStatus};
use ruviform_plugin_core::wire::WIRE_MAX_FRAMES;
use ruviform_plugin_core::{
    BufferView, BufferViewMut, ErasedPlugin, Event, EventKind, EventList, GuiHandle, HostError,
    ParamEvent, ParamFlags, ParamId, ParamInfo, ParentLease, Plugin, PluginDescriptor,
    PluginFactory, PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult,
    ProcessStatus, TransportInfo, erase,
};
use vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0, BusDirection, BusDirections_, BusInfo, Chord,
    Event as VstEvent, FrameRate, IAttributeList, IAttributeList_iid, IAttributeListTrait,
    IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentHandlerTrait,
    IComponentTrait, IConnectionPoint, IConnectionPointTrait, IEditController,
    IEditControllerTrait, IEventList, IHostApplication, IHostApplicationTrait, IMessage,
    IMessage_iid, IMessageTrait, IMidiMapping, IParamValueQueue, IParamValueQueueTrait,
    IParameterChanges, IParameterChangesTrait, MediaTypes_, ParamID, ParamValue, ParameterInfo,
    ParameterInfo_::ParameterFlags_, ProcessContext as VstProcessContext, ProcessContext_,
    ProcessData, ProcessModes_, ProcessSetup, String128, SymbolicSampleSizes_, TChar, ViewType,
};
use vst3::Steinberg::{
    FUnknown, FUnknown_iid, IPlugFrame, IPlugFrameTrait, IPlugView, IPlugViewTrait,
    IPluginBaseTrait, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3,
    IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, TUID,
    ViewRect, char8, kInvalidArgument, kNotImplemented, kResultFalse, kResultOk, kResultTrue,
    tresult,
};
use vst3::{Class, ComPtr, ComWrapper, Interface};

use crate::block;
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
/// `attached`. `resizeView` is the plugin-initiated resize request:
/// the bridge answers the honest `kResultFalse` because container
/// geometry is owned by the slot side (`NativeSurfaceSlot`/
/// `set_geometry`) and no neutral channel carries a plugin-requested
/// resize back to it — acknowledging would fake a resize the bridge
/// never performs (`docs/designs/issue-389-vst3-l0-editor-attach.md`
/// D2).
struct BridgePlugFrame;

impl Class for BridgePlugFrame {
    type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for BridgePlugFrame {
    #[expect(unsafe_code)]
    unsafe fn resizeView(&self, _view: *mut IPlugView, _new_size: *mut ViewRect) -> tresult {
        // `kResultFalse` — a defined refusal: the plugin keeps its
        // current size; the bridge cannot move the consumer-owned
        // container.
        kResultFalse
    }
}

/// Minimal `IComponentHandler` — acknowledge-only. The fixture's
/// `setComponentHandler` is non-retaining (`controller.rs`); routing
/// `performEdit`/`restartComponent` back into the component is a v1
/// defined boundary the bridge does not carry (no connection point —
/// the runner's only param path is `inputParameterChanges`).
///
/// `beginEdit`/`performEdit`/`endEdit` answer `kResultOk` — a refused
/// edit could wedge a plugin that gates its UI on the ack.
/// `restartComponent` answers `kNotImplemented` instead: `kResultOk`
/// would promise the flag-requested resend (param values, component
/// state, latency) that this handler never performs — the dishonest
/// ack is worse than the refusal.
struct BridgeComponentHandler;

impl Class for BridgeComponentHandler {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for BridgeComponentHandler {
    #[expect(unsafe_code)]
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn performEdit(&self, _id: ParamID, _value_normalized: ParamValue) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn endEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    #[expect(unsafe_code)]
    unsafe fn restartComponent(&self, _flags: i32) -> tresult {
        // `kResultOk` would ack a restart the bridge never performs —
        // the flag-requested resend (param values / component state)
        // has no connection point in v1, so the honest answer is the
        // refusal.
        kNotImplemented
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
            let result = unsafe { view.removed() };
            debug_assert_eq!(result, kResultOk, "view.removed() declined");
            let _ = unsafe { view.setFrame(ptr::null_mut()) };
            drop(view);
        }
        drop(self.plug_frame.take());
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
            let handler = ComWrapper::new(BridgeComponentHandler);
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
        let frame_wrapper = ComWrapper::new(BridgePlugFrame);
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
    use ruviform_plugin_core::{Event, EventKind, HostInput, HostOutput, ParamEvent, ParamValue};
    use std::path::PathBuf;

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
    fn unsupported_state_verbs_are_defaults() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        assert_eq!(plugin.save_state(), Err(HostError::Unsupported));
        assert_eq!(
            plugin.restore_state(&[1, 2, 3]),
            Err(HostError::Unsupported)
        );
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
}
