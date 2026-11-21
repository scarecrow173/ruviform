//! The `test_editor` plugin-side `IPlugView`: a real native editor
//! surface — the plugin-fixture half of Issue #630 AC-4 ("Native
//! Editor が実 child HWND を生成できる").
//!
//! ## Contract (what the production host's `open_gui` requires)
//!
//! `Vst3Plugin::open_gui` (Issue #389 substrate, unchanged) calls, in
//! order: `createView("editor")` on the paired controller →
//! `isPlatformTypeSupported("HWND")` which must answer `kResultTrue`
//! (not `kResultOk`) → `setFrame(BridgePlugFrame)` →
//! `attached(parent, "HWND")`. `close_view` calls `removed()`
//! (debug-asserted `kResultOk`) then `setFrame(null)`. Every other
//! platform type, a null/invalid parent, and non-Windows builds are
//! honest `kResultFalse` rejections — success is never faked, so a
//! fake-lane lease payload (an opaque `u64`, not a real HWND) fails
//! the `IsWindow` gate instead of passing.
//!
//! ## Windows child window
//!
//! On Windows, `attached` creates a real `WS_CHILD | WS_VISIBLE |
//! WS_CLIPSIBLINGS` HWND under the host-supplied parent — a solid
//! fill painted through the class background brush. The class is
//! registered once per module with `lpfnWndProc = DefWindowProcW`
//! deliberately: a cdylib-registered window class **outlives the
//! module** (Windows does not unregister DLL classes on unload), so
//! any `lpfnWndProc` pointing into this binary would dangle after
//! `FreeLibrary` and crash the host on the next message dispatch —
//! including on reload, where `GetClassInfoW` would happily adopt
//! the stale class. `DefWindowProcW` lives in `user32`, which never
//! unloads, so the persisted class stays fully functional across
//! module unload/reload; the one-time brush leak is a GDI object,
//! not module code. Registration still follows the
//! `ruviform-poc-native-dock` probe precedent (once-per-module,
//! `GetClassInfoW` confirmation) and there is **no message pump or
//! thread of our own** (the host calls `attached` on the thread
//! that owns the parent HWND — the #389 D1 thread-affinity
//! contract). Win32 calls are confined to the
//! `#[cfg(target_os = "windows")]` `win32` module behind
//! `#[expect(unsafe_code)]` + SAFETY comments; off Windows the same
//! `win32` shim answers `None`/no-op so `attached` converges to the
//! defined `kResultFalse` rejection and the cdylib still builds.
//!
//! [`TestEditorView`] is created only through
//! `IEditController::createView("editor")` — it is not a factory
//! class. Interior mutability follows the `controller.rs`
//! `AtomicU64`-style pattern: trait methods take `&self`, so state
//! lives in a `RefCell` guarded by `try_borrow_mut` (re-entrant calls
//! reject with `kResultFalse`, the same policy `component.rs`
//! applies to `ComponentState`).

use std::cell::RefCell;
use std::ffi::c_void;

use vst3::Steinberg::{
    FIDString, IPlugFrame, IPlugView, IPlugViewTrait, ViewRect, kInvalidArgument, kResultFalse,
    kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComPtr, ComRef};

use crate::strings::cstring_eq;

/// The editor's preferred size — the deterministic fixed rect
/// `getSize` reports until `onSize` updates it.
const DEFAULT_WIDTH: i32 = 640;
const DEFAULT_HEIGHT: i32 = 480;

/// Interior-mutable view state — every `IPlugViewTrait` method takes
/// `&self`, so the fields live behind `RefCell`.
struct ViewState {
    /// The attached child HWND as a `usize` (`None` = detached).
    /// Only ever `Some` on Windows — the non-Windows `win32` shim
    /// never produces one.
    hwnd: Option<usize>,
    /// The `IPlugFrame` the host supplied via `setFrame` — stored as
    /// an owned `ComPtr` (addRef'd on store, released on replace or
    /// drop). The host keeps its own reference; `setFrame(null)`
    /// releases ours.
    frame: Option<ComPtr<IPlugFrame>>,
    /// The last reported size — `getSize` echoes it, `onSize`
    /// replaces it.
    size: ViewRect,
    /// Attach generation — stamped into the child HWND's window text
    /// (`rvf-test-editor-attach-{n}`) so an observer can prove a
    /// detach/`removed` + `attached` cycle really recreated the
    /// surface (an HWND value alone can be reused by Windows).
    attach_seq: u32,
    /// The recorded `onFocus`/`onKeyDown`/`onKeyUp` tokens —
    /// `f:{state}`, `kd:{key:04x}:{key_code:04x}:{modifiers:04x}`,
    /// `ku:{…}` — appended to the window title so an observer can
    /// prove the input/focus verbs reached the editor through the
    /// same `GetWindowTextW` channel the attach generation uses
    /// (the fixture has no other observable output).
    events: Vec<String>,
    /// The plugin-owned popup window as a `usize` (`None` = closed).
    /// `test_editor` ships no host-visible popup verb — `IPlugView`
    /// has none — so the fixture opens its own transient window the
    /// way a real plugin opens a context menu: on user input. The
    /// designated key is `key_code == 0x50` (`VK_P`) in `onKeyDown`.
    popup: Option<usize>,
    /// Popup generation — stamped into the popup HWND's window text
    /// (`rvf-test-editor-popup-{n}`) so an observer can prove a
    /// close/open cycle really recreated the surface.
    popup_seq: u32,
}

/// The `key_code` that toggles the plugin-owned popup — `VK_P`
/// (`0x50`). A real plugin opens its popup/menu on user input; the
/// fixture's only input verb is `onKeyDown`, so the designated key
/// stands in for the menu gesture.
const POPUP_VK: i16 = 0x50;

/// `test_editor`'s plug view — one instance per
/// `createView("editor")` call; the host owns the returned COM
/// pointer and drives the whole attach/remove lifecycle.
pub(crate) struct TestEditorView {
    state: RefCell<ViewState>,
}

impl Class for TestEditorView {
    type Interfaces = (IPlugView,);
}

impl TestEditorView {
    /// A fresh detached view reporting the default 640x480 size.
    pub(crate) fn new() -> Self {
        Self {
            state: RefCell::new(ViewState {
                hwnd: None,
                frame: None,
                size: ViewRect {
                    left: 0,
                    top: 0,
                    right: DEFAULT_WIDTH,
                    bottom: DEFAULT_HEIGHT,
                },
                attach_seq: 0,
                events: Vec::new(),
                popup: None,
                popup_seq: 0,
            }),
        }
    }

    /// Records one input/focus token and restamps the child HWND's
    /// window text — the OS-observable effect the real-window lane's
    /// `effect_confirmed` oracle reads through `GetWindowTextW`. A
    /// re-entrant call rejects `kResultFalse` like every other state
    /// mutation here.
    fn record(&self, token: String) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        state.events.push(token);
        if let Some(hwnd) = state.hwnd {
            win32::set_title(hwnd, &state.window_title());
        }
        kResultOk
    }

    /// Toggles the plugin-owned popup: open creates a real
    /// `WS_POPUP` HWND titled `rvf-test-editor-popup-{seq}` owned by
    /// the editor's root ancestor when one exists; close destroys
    /// it. The `po:{seq}`/`pc:{seq}` tokens land in the same event
    /// log the `kd:` tokens use — the verb dispatch is recorded
    /// whether or not this build can create HWNDs (off Windows the
    /// `win32` shim answers `None` and the log still shows the verb
    /// ran, the same split `attached` uses).
    fn popup_toggle(&self) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        if let Some(popup) = state.popup.take() {
            win32::destroy(popup);
            let seq = state.popup_seq;
            state.events.push(format!("pc:{seq}"));
        } else {
            state.popup_seq = state.popup_seq.wrapping_add(1);
            let seq = state.popup_seq;
            state.popup = win32::popup(state.hwnd, &format!("rvf-test-editor-popup-{seq}"));
            state.events.push(format!("po:{seq}"));
        }
        if let Some(hwnd) = state.hwnd {
            win32::set_title(hwnd, &state.window_title());
        }
        kResultOk
    }
}

impl ViewState {
    /// The window text stamped onto the child HWND: the attach
    /// generation tag plus the recorded input/focus log —
    /// `rvf-test-editor-attach-{n}[|{ev},{ev},…]`.
    fn window_title(&self) -> String {
        let mut title = format!("rvf-test-editor-attach-{}", self.attach_seq);
        if !self.events.is_empty() {
            title.push('|');
            title.push_str(&self.events.join(","));
        }
        title
    }
}

impl IPlugViewTrait for TestEditorView {
    /// Only the Windows HWND platform is supported — the fixture's
    /// editor is a real child window. Every other platform string is
    /// an honest `kResultFalse`.
    ///
    /// The host's `open_gui` requires exactly `kResultTrue` here —
    /// `kResultOk` would be misread as unsupported.
    #[expect(unsafe_code)]
    unsafe fn isPlatformTypeSupported(&self, r#type: FIDString) -> tresult {
        // SAFETY: `type` is a VST3 `FIDString` — a valid
        // NUL-terminated `char8` string or null per contract.
        if unsafe { cstring_eq(r#type, "HWND") } {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    /// Attaches the view under `parent`: on Windows this creates the
    /// real `WS_CHILD` HWND; everywhere else — and for null, non-HWND,
    /// already-attached, or window-creation-failure inputs — it is a
    /// defined `kResultFalse` rejection, never a faked success.
    #[expect(unsafe_code)]
    unsafe fn attached(&self, parent: *mut c_void, r#type: FIDString) -> tresult {
        // SAFETY: `type` is a valid `FIDString` per contract.
        if !unsafe { cstring_eq(r#type, "HWND") } {
            return kResultFalse;
        }
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        if state.hwnd.is_some() {
            // Already attached — a second `attached` without an
            // intervening `removed` is a protocol violation.
            return kResultFalse;
        }
        state.attach_seq = state.attach_seq.wrapping_add(1);
        let title = state.window_title();
        match win32::attach(parent, &title) {
            Some(hwnd) => {
                state.hwnd = Some(hwnd);
                kResultOk
            }
            None => kResultFalse,
        }
    }

    /// Detaches the view: destroys the child HWND and any open popup
    /// and always reports `kResultOk` — the host debug-asserts this
    /// result, so declining would turn a benign double-`removed`
    /// into a failure.
    #[expect(unsafe_code)]
    unsafe fn removed(&self) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        if let Some(popup) = state.popup.take() {
            win32::destroy(popup);
        }
        if let Some(hwnd) = state.hwnd.take() {
            win32::destroy(hwnd);
        }
        kResultOk
    }

    /// Acknowledges wheel input — the fixture has no input surface;
    /// the notification is accepted and ignored.
    #[expect(unsafe_code)]
    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultOk
    }

    /// Records the key-down — the token lands in the event log the
    /// child HWND's window text carries (`kd:{key}:{key_code}:{mods}`
    /// as four-digit hex), making the delivery OS-observable. The
    /// designated popup key (`VK_P`) additionally toggles the
    /// plugin-owned popup window and logs `po:{seq}`/`pc:{seq}`.
    #[expect(unsafe_code)]
    unsafe fn onKeyDown(&self, key: u16, key_code: i16, modifiers: i16) -> tresult {
        let result = self.record(format!("kd:{key:04x}:{key_code:04x}:{modifiers:04x}"));
        if result == kResultOk && key_code == POPUP_VK {
            self.popup_toggle()
        } else {
            result
        }
    }

    /// Records the key-up — same log shape as `onKeyDown`.
    #[expect(unsafe_code)]
    unsafe fn onKeyUp(&self, key: u16, key_code: i16, modifiers: i16) -> tresult {
        self.record(format!("ku:{key:04x}:{key_code:04x}:{modifiers:04x}"))
    }

    /// Reports the current size — the default 640x480 rect until
    /// `onSize` updates it.
    #[expect(unsafe_code)]
    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        if size.is_null() {
            return kInvalidArgument;
        }
        let Ok(state) = self.state.try_borrow() else {
            return kResultFalse;
        };
        // SAFETY: out-param checked non-null; `ViewRect` is a POD the
        // caller owns for the call.
        unsafe { *size = state.size };
        kResultOk
    }

    /// Records the container-reported size — `getSize` echoes it
    /// afterwards.
    #[expect(unsafe_code)]
    unsafe fn onSize(&self, new_size: *mut ViewRect) -> tresult {
        if new_size.is_null() {
            return kInvalidArgument;
        }
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        // SAFETY: in-param — read the caller's rect once.
        let rect = unsafe { *new_size };
        state.size = rect;
        // The fixture's child HWND fills the reported size — the
        // observable effect a real-window lane can verify through
        // `GetWindowRect` (the plugin owns its window; the host only
        // reports the size).
        if let Some(hwnd) = state.hwnd {
            win32::resize(hwnd, rect.right - rect.left, rect.bottom - rect.top);
        }
        kResultOk
    }

    /// Records the focus change — `f:{state}` lands in the event log
    /// the child HWND's window text carries. `TBool` is `uint8` in
    /// the bindings.
    #[expect(unsafe_code)]
    unsafe fn onFocus(&self, state: u8) -> tresult {
        self.record(format!("f:{state}"))
    }

    /// Retains the host's `IPlugFrame` (or releases ours on null) —
    /// the caller keeps its own reference either way.
    #[expect(unsafe_code)]
    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return kResultFalse;
        };
        if frame.is_null() {
            // Drops the stored `ComPtr` → releases our reference.
            state.frame = None;
            return kResultOk;
        }
        // SAFETY: `frame` is a valid `IPlugFrame` pointer per the
        // `setFrame` contract (non-null here). `ComRef` borrows it
        // without touching the refcount; `to_com_ptr` addRefs so our
        // stored `ComPtr` owns exactly one reference.
        let Some(frame) = (unsafe { ComRef::from_raw(frame) }) else {
            return kResultFalse;
        };
        state.frame = Some(frame.to_com_ptr());
        kResultOk
    }

    /// The fixture's editor genuinely resizes — `onSize` moves the
    /// child HWND to the reported rect, so the honest answer is
    /// `kResultTrue`. (Plugin-initiated `resizeView` on the host's
    /// `IPlugFrame` still declines — container geometry belongs to
    /// the slot — but that is the opposite direction.)
    #[expect(unsafe_code)]
    unsafe fn canResize(&self) -> tresult {
        kResultTrue
    }

    /// Accepts the proposed rect unchanged — the fixture has no size
    /// constraints to enforce.
    #[expect(unsafe_code)]
    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        if rect.is_null() {
            return kInvalidArgument;
        }
        kResultOk
    }
}

/// Windows child-window management — the only Win32 surface in this
/// crate. The window class deliberately registers
/// `lpfnWndProc = DefWindowProcW` (not a function inside this cdylib):
/// classes registered by a DLL persist after the module unloads, and
/// a module-local `wnd_proc` would dangle — crashing the host on the
/// next dispatched message, including after a reload that adopts the
/// stale class via `GetClassInfoW`. `DefWindowProcW` fills
/// `WM_ERASEBKGND` with the class `hbrBackground`, which produces the
/// observable solid fill with zero module-owned code in the class.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
mod win32 {
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::Once;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows_sys::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleHandleExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GA_ROOT,
        GetAncestor, GetClassInfoW, GetClientRect, IsWindow, RegisterClassW, SWP_NOMOVE,
        SWP_NOZORDER, SetWindowPos, SetWindowTextW, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS, WS_POPUP,
        WS_VISIBLE,
    };

    /// Solid-fill color — green, distinct from the `PoC` probe's red so
    /// a `test_editor` surface is visually identifiable.
    const EDITOR_COLOR: u32 = 0x00_00_FF_00;

    /// This module's `HINSTANCE`, captured at registration — reused
    /// as the `hInstance` argument for `CreateWindowExW` so window
    /// creation is consistent with the class registration.
    static MODULE_H: AtomicIsize = AtomicIsize::new(0);

    /// Set once class registration succeeded (or the class was found
    /// already registered — including a stale copy left behind by a
    /// previous load of this module, which is safe precisely because
    /// the class's `wnd_proc` is `DefWindowProcW`).
    static REGISTERED: AtomicBool = AtomicBool::new(false);

    /// The wide class name — `"RuviformTestEditor"`, distinct from the
    /// `PoC` probe's `"RvfProbeEditor"`.
    fn class_name() -> Vec<u16> {
        "RuviformTestEditor"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    /// This module's own `HINSTANCE` — resolved from an address
    /// inside this binary (`class_name`), never `GetModuleHandleW(NULL)`
    /// (which would report the host EXE). `UNCHANGED_REFCOUNT` keeps
    /// the lookup free of refcount side effects.
    ///
    /// # Safety
    /// `class_name` is a function in this module, so its address is a
    /// valid `FROM_ADDRESS` anchor for `GetModuleHandleExW`.
    fn module_instance() -> HINSTANCE {
        let mut module: HINSTANCE = ptr::null_mut();
        // SAFETY: `class_name`'s code address lies inside this
        // module; `module` is a valid out-pointer.
        unsafe {
            let found = GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                class_name as *const u16,
                &raw mut module,
            );
            if found == 0 { ptr::null_mut() } else { module }
        }
    }

    /// Registers the editor window class once per module. A second
    /// copy of this module in the same process — or a stale class
    /// left by an unloaded copy — hits `ERROR_CLASS_ALREADY_EXISTS`
    /// on `RegisterClassW`; the class is still usable because its
    /// `wnd_proc` is `DefWindowProcW`, confirmed via `GetClassInfoW`
    /// before being accepted.
    fn ensure_registered() -> bool {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let name = class_name();
            // SAFETY: the class name outlives the registration via
            // `mem::forget` (Windows does not copy `lpszClassName`);
            // the brush intentionally leaks — it must outlive every
            // editor window the class creates, including windows that
            // outlive this module.
            unsafe {
                let brush = CreateSolidBrush(EDITOR_COLOR);
                let hinstance = module_instance();
                MODULE_H.store(hinstance as isize, Ordering::Relaxed);
                let class = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    // `DefWindowProcW` — never a pointer into this
                    // cdylib (see module docs: the class outlives us).
                    lpfnWndProc: Some(DefWindowProcW),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: ptr::null_mut(),
                    hCursor: ptr::null_mut(),
                    hbrBackground: brush,
                    lpszMenuName: ptr::null(),
                    lpszClassName: name.as_ptr(),
                };
                if RegisterClassW(&raw const class) != 0 {
                    REGISTERED.store(true, Ordering::Release);
                } else {
                    // Possibly already registered by this or another
                    // copy of this module — verify the class really
                    // exists before accepting it.
                    let mut existing: WNDCLASSW = std::mem::zeroed();
                    let exists = GetClassInfoW(hinstance, name.as_ptr(), &raw mut existing) != 0;
                    REGISTERED.store(exists, Ordering::Release);
                }
            }
            std::mem::forget(name);
        });
        REGISTERED.load(Ordering::Acquire)
    }

    /// Creates the editor child HWND under `parent`, filling the
    /// parent's client area. `parent` must be a live window handle on
    /// the calling thread (the `open_gui` thread-affinity contract);
    /// null, dead, or non-window handles are rejected before any
    /// Win32 mutation. `title` becomes the window text — the
    /// attach-generation tag an observer reads back through
    /// `GetWindowTextW`.
    ///
    /// # Returns
    /// `Some` child HWND handle (`usize`) on success, `None` on any
    /// defined rejection — the caller maps `None` to `kResultFalse`.
    pub(super) fn attach(parent: *mut c_void, title: &str) -> Option<usize> {
        // SAFETY: `IsWindow` is a pure handle-validity query — safe on
        // any pointer value including null.
        if parent.is_null() || unsafe { IsWindow(parent) } == 0 {
            return None;
        }
        if !ensure_registered() {
            return None;
        }
        let class = class_name();
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `parent` passed the `IsWindow` gate; the class and
        // title outlive the synchronous call; the returned HWND is
        // owned by this view until `destroy`.
        let child = unsafe {
            let mut rc = RECT::default();
            GetClientRect(parent, &raw mut rc);
            CreateWindowExW(
                0,
                class.as_ptr(),
                title_w.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                parent,
                ptr::null_mut(),
                MODULE_H.load(Ordering::Relaxed) as HINSTANCE,
                ptr::null(),
            )
        };
        if child.is_null() {
            None
        } else {
            Some(child as usize)
        }
    }

    /// Resizes the child HWND to `width`×`height` device pixels,
    /// keeping its position and z-order — the observable effect of
    /// the plugin's `onSize` hook. Best-effort: a dead handle is a
    /// harmless `SetWindowPos` error.
    pub(super) fn resize(hwnd: usize, width: i32, height: i32) {
        // SAFETY: `hwnd` is the child this module created; `SetWindowPos`
        // on an already-dead handle is a defined no-op error.
        unsafe {
            SetWindowPos(
                hwnd as HWND,
                ptr::null_mut(),
                0,
                0,
                width,
                height,
                SWP_NOMOVE | SWP_NOZORDER,
            );
        }
    }

    /// Restamps the child HWND's window text — the event-log half of
    /// the title an observer reads back through `GetWindowTextW`.
    /// Best-effort: a dead handle is a harmless `SetWindowTextW`
    /// error.
    pub(super) fn set_title(hwnd: usize, title: &str) {
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `hwnd` is the child this module created; on a dead
        // handle the call is a defined no-op error.
        unsafe {
            SetWindowTextW(hwnd as HWND, title_w.as_ptr());
        }
    }

    /// Destroys the child HWND — `DestroyWindow` on an already-dead
    /// handle is a harmless error, so this is idempotent-safe.
    pub(super) fn destroy(hwnd: usize) {
        // SAFETY: `hwnd` is the child this module created; destroying
        // an already-dead handle is a defined no-op error.
        unsafe {
            DestroyWindow(hwnd as HWND);
        }
    }

    /// Creates the plugin-owned popup window — a `WS_POPUP` HWND
    /// titled `rvf-test-editor-popup-{seq}`. `anchor` is the editor's
    /// child HWND when attached: the popup is owned by that window's
    /// root ancestor so it stays above the host's top-level window
    /// and dies with it; an unattached view creates an unowned popup
    /// (still a real HWND the observer can find). Returns `None` on
    /// any defined failure — the caller still logs the verb.
    pub(super) fn popup(anchor: Option<usize>, title: &str) -> Option<usize> {
        if !ensure_registered() {
            return None;
        }
        // SAFETY: `IsWindow`/`GetAncestor` are pure handle queries —
        // safe on any value including a dead or bogus anchor.
        let owner = anchor
            .filter(|hwnd| unsafe { IsWindow(*hwnd as HWND) } != 0)
            .map(|hwnd| unsafe { GetAncestor(hwnd as HWND, GA_ROOT) });
        let class = class_name();
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: the class and title outlive the synchronous call;
        // `owner` is a live window or null (unowned popup). The
        // returned HWND is owned by this view until `destroy`.
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title_w.as_ptr(),
                WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                200,
                100,
                owner.unwrap_or(ptr::null_mut()),
                ptr::null_mut(),
                MODULE_H.load(Ordering::Relaxed) as HINSTANCE,
                ptr::null(),
            )
        };
        if window.is_null() {
            None
        } else {
            Some(window as usize)
        }
    }

    /// Whether `hwnd` currently names a live window — the test-side
    /// existence check for popup/editor surfaces.
    #[cfg(test)]
    pub(super) fn is_window(hwnd: usize) -> bool {
        // SAFETY: `IsWindow` is a pure handle-validity query — safe
        // on any value.
        unsafe { IsWindow(hwnd as HWND) != 0 }
    }
}

/// Non-Windows shim — no HWND surface exists off Windows, so
/// `attach` always answers `None` (the caller's defined rejection)
/// and `destroy` is unreachable but defined.
#[cfg(not(target_os = "windows"))]
mod win32 {
    use std::ffi::c_void;

    pub(super) fn attach(_parent: *mut c_void, _title: &str) -> Option<usize> {
        None
    }

    pub(super) fn resize(_hwnd: usize, _width: i32, _height: i32) {}

    pub(super) fn set_title(_hwnd: usize, _title: &str) {}

    pub(super) fn destroy(_hwnd: usize) {}

    pub(super) fn popup(_anchor: Option<usize>, _title: &str) -> Option<usize> {
        None
    }

    #[cfg(test)]
    pub(super) fn is_window(_hwnd: usize) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;
    use vst3::Steinberg::Vst::ViewType;
    use vst3::{ComPtr, ComWrapper};

    /// `createView`-equivalent: a fresh view behind its real COM
    /// pointer, driven through the vtbl — the same dispatch a host
    /// performs.
    fn view_ptr() -> ComPtr<IPlugView> {
        let wrapper = ComWrapper::new(TestEditorView::new());
        wrapper
            .to_com_ptr::<IPlugView>()
            .expect("IPlugView is implemented")
    }

    /// `isPlatformTypeSupported` answers `kResultTrue` only for
    /// `"HWND"` — the exact value `open_gui` requires — and
    /// `kResultFalse` for every other type including null.
    #[expect(unsafe_code)]
    #[test]
    fn platform_support_is_hwnd_only() {
        let view = view_ptr();
        // SAFETY: `view` is a live COM pointer; the FIDString args are
        // the SDK's own NUL-terminated constants or null.
        unsafe {
            assert_eq!(
                view.isPlatformTypeSupported(vst3::Steinberg::kPlatformTypeHWND),
                kResultTrue
            );
            assert_eq!(
                view.isPlatformTypeSupported(vst3::Steinberg::kPlatformTypeNSView),
                kResultFalse
            );
            assert_eq!(
                view.isPlatformTypeSupported(vst3::Steinberg::kPlatformTypeX11EmbedWindowID),
                kResultFalse
            );
            assert_eq!(
                view.isPlatformTypeSupported(vst3::Steinberg::kPlatformTypeUIView),
                kResultFalse
            );
            assert_eq!(view.isPlatformTypeSupported(ptr::null()), kResultFalse);
        }
    }

    /// `getSize` reports the deterministic default rect; `onSize`
    /// replaces it; null out-params are `kInvalidArgument`.
    #[expect(unsafe_code)]
    #[test]
    fn size_round_trips_through_on_size() {
        let view = view_ptr();
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `rect` is a valid out-param.
        unsafe {
            assert_eq!(view.getSize(&raw mut rect), kResultOk);
            assert_eq!(rect.right, DEFAULT_WIDTH);
            assert_eq!(rect.bottom, DEFAULT_HEIGHT);

            let mut bigger = ViewRect {
                left: 0,
                top: 0,
                right: 800,
                bottom: 600,
            };
            assert_eq!(view.onSize(&raw mut bigger), kResultOk);
            assert_eq!(view.getSize(&raw mut rect), kResultOk);
            assert_eq!(rect.right, 800);
            assert_eq!(rect.bottom, 600);

            assert_eq!(view.getSize(ptr::null_mut()), kInvalidArgument);
            assert_eq!(view.onSize(ptr::null_mut()), kInvalidArgument);
        }
    }

    /// The designated popup key (`VK_P` on `key_code`) toggles the
    /// plugin-owned popup: `po:{seq}`/`pc:{seq}` tokens land in the
    /// event log and, on Windows, a real `WS_POPUP` HWND exists
    /// between them. Every other key only records `kd:`.
    #[expect(unsafe_code)]
    #[test]
    fn popup_key_toggles_plugin_owned_window() {
        let view = TestEditorView::new();
        // SAFETY: trait methods on a live view instance; the args
        // are plain scalar values.
        unsafe {
            // A non-popup key — records `kd:` only.
            view.onKeyDown(0x41, 0x41, 0);
            assert_eq!(view.state.borrow().events, ["kd:0041:0041:0000"]);

            // Popup open — token plus, on Windows, a live HWND.
            view.onKeyDown(0x50, POPUP_VK, 0);
            {
                let state = view.state.borrow();
                assert!(state.events.contains(&"po:1".to_string()));
                assert_eq!(state.popup.is_some(), cfg!(windows));
                if let Some(popup) = state.popup {
                    assert!(win32::is_window(popup));
                }
            }
            // Popup close — token, HWND destroyed.
            view.onKeyDown(0x50, POPUP_VK, 0);
            {
                let state = view.state.borrow();
                assert!(state.events.contains(&"pc:1".to_string()));
                assert!(state.popup.is_none());
            }
            // Re-open advances the generation.
            view.onKeyDown(0x50, POPUP_VK, 0);
            assert!(view.state.borrow().events.contains(&"po:2".to_string()));
            // `removed` tears the open popup down.
            view.removed();
            assert!(view.state.borrow().popup.is_none());
        }
    }

    /// The `po:`/`pc:` verb tokens are recorded even when the popup
    /// HWND never materialized — the toggle's open/close oracle is
    /// the logical open state, not `state.popup` (Issue #855: an
    /// HWND-based oracle left `pc:` unreachable wherever
    /// `win32::popup` answers `None` — every non-Windows build, and
    /// any `CreateWindowExW` failure on Windows). Here the missing
    /// surface is simulated by taking and destroying the real HWND
    /// so `state.popup` is `None`, the same state the non-Windows
    /// stub produces.
    #[expect(unsafe_code)]
    #[test]
    fn popup_verbs_are_recorded_without_popup_hwnd() {
        let view = TestEditorView::new();
        // SAFETY: trait methods on a live view instance; the args
        // are plain scalar values.
        unsafe {
            for _ in 0..2 {
                // Open — `po:{seq}`; then drop the surface so the
                // next press sees `popup == None`, exactly what a
                // non-Windows build sees.
                view.onKeyDown(0x50, POPUP_VK, 0);
                if let Some(popup) = view.state.borrow_mut().popup.take() {
                    win32::destroy(popup);
                }
                // Close — `pc:{seq}` must be recorded even though no
                // HWND existed at toggle time.
                view.onKeyDown(0x50, POPUP_VK, 0);
            }
            let state = view.state.borrow();
            let popup_tokens: Vec<&str> = state
                .events
                .iter()
                .map(String::as_str)
                .filter(|token| token.starts_with('p'))
                .collect();
            assert_eq!(popup_tokens, ["po:1", "pc:1", "po:2", "pc:2"]);
        }
    }

    /// `removed` tears the popup down and resets the logical open
    /// state, so the next popup key starts a fresh `po:{seq}` cycle —
    /// a stale open flag would mis-record it as `pc:`.
    #[expect(unsafe_code)]
    #[test]
    fn removed_resets_popup_open_state() {
        let view = TestEditorView::new();
        // SAFETY: trait methods on a live view instance; the args
        // are plain scalar values.
        unsafe {
            view.onKeyDown(0x50, POPUP_VK, 0);
            view.removed();
            view.onKeyDown(0x50, POPUP_VK, 0);
            let state = view.state.borrow();
            let popup_tokens: Vec<&str> = state
                .events
                .iter()
                .map(String::as_str)
                .filter(|token| token.starts_with('p'))
                .collect();
            assert_eq!(popup_tokens, ["po:1", "po:2"]);
        }
    }

    /// `canResize` honestly accepts (the fixture's `onSize` resizes
    /// the child HWND); `checkSizeConstraint` accepts a valid rect
    /// and rejects null.
    #[expect(unsafe_code)]
    #[test]
    fn resize_verbs_are_honest() {
        let view = view_ptr();
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        // SAFETY: `rect` is a valid in/out-param.
        unsafe {
            assert_eq!(view.canResize(), kResultTrue);
            assert_eq!(view.checkSizeConstraint(&raw mut rect), kResultOk);
            assert_eq!(view.checkSizeConstraint(ptr::null_mut()), kInvalidArgument);
        }
    }

    /// Notification verbs acknowledge with `kResultOk`.
    #[expect(unsafe_code)]
    #[test]
    fn notification_verbs_acknowledge() {
        let view = view_ptr();
        // SAFETY: no pointers involved.
        unsafe {
            assert_eq!(view.onWheel(1.0), kResultOk);
            assert_eq!(view.onKeyDown('a' as u16, 0, 0), kResultOk);
            assert_eq!(view.onKeyUp('a' as u16, 0, 0), kResultOk);
            assert_eq!(view.onFocus(1), kResultOk);
        }
    }

    /// `onFocus`/`onKeyDown`/`onKeyUp` append compact tokens to the
    /// event log in call order — the same order a `GetWindowTextW`
    /// observer reads them off the child HWND's title (detached here,
    /// so the log is the evidence).
    #[expect(unsafe_code)]
    #[test]
    fn input_focus_verbs_record_event_tokens_in_order() {
        let view = TestEditorView::new();
        // SAFETY: `IPlugViewTrait` methods take `&self` — driving the
        // view directly is the same dispatch the COM vtbl performs.
        unsafe {
            assert_eq!(view.onFocus(1), kResultOk);
            assert_eq!(view.onKeyDown(0x0061, 0x41, 0x0001), kResultOk);
            assert_eq!(view.onKeyUp(0x0061, 0x41, 0x0000), kResultOk);
            assert_eq!(view.onFocus(0), kResultOk);
        }
        let state = view.state.borrow();
        assert_eq!(
            state.events.as_slice(),
            &["f:1", "kd:0061:0041:0001", "ku:0061:0041:0000", "f:0"]
        );
        assert_eq!(
            state.window_title(),
            "rvf-test-editor-attach-0|f:1,kd:0061:0041:0001,ku:0061:0041:0000,f:0"
        );
    }

    /// `attached` is an honest gate: wrong platform type, null parent,
    /// and a non-window payload all reject with `kResultFalse` — on
    /// every OS. `removed` is idempotent `kResultOk`.
    #[expect(unsafe_code)]
    #[test]
    fn attach_rejects_invalid_parents_honestly() {
        let view = view_ptr();
        let hwnd_type = vst3::Steinberg::kPlatformTypeHWND;
        // SAFETY: `view` is live; the FIDString args are the SDK's own
        // NUL-terminated constants; the bogus parent is never
        // dereferenced — `IsWindow` is a pure handle query.
        unsafe {
            // Wrong platform type — rejected on the type gate before
            // any window work, regardless of the parent value.
            assert_eq!(
                view.attached(ptr::null_mut(), vst3::Steinberg::kPlatformTypeNSView),
                kResultFalse
            );
            assert_eq!(
                view.attached(
                    0xDEAD_BEEF_usize as *mut c_void,
                    vst3::Steinberg::kPlatformTypeNSView
                ),
                kResultFalse
            );
            // HWND type but null parent.
            assert_eq!(view.attached(ptr::null_mut(), hwnd_type), kResultFalse);
            // HWND type but a payload that is not a live window —
            // `IsWindow` rejects it on Windows; the non-Windows shim
            // rejects unconditionally.
            assert_eq!(
                view.attached(0xDEAD_BEEF_usize as *mut c_void, hwnd_type),
                kResultFalse
            );
            // `removed` on a never-attached view is still `kResultOk`.
            assert_eq!(view.removed(), kResultOk);
            assert_eq!(view.removed(), kResultOk);
        }
    }

    /// `setFrame` accepts null (releasing any stored frame) and a
    /// real `IPlugFrame` — both `kResultOk`.
    #[expect(unsafe_code)]
    #[test]
    fn set_frame_accepts_null_and_real_frame() {
        struct TestFrame;
        impl Class for TestFrame {
            type Interfaces = (IPlugFrame,);
        }
        impl vst3::Steinberg::IPlugFrameTrait for TestFrame {
            #[expect(unsafe_code)]
            unsafe fn resizeView(
                &self,
                _view: *mut IPlugView,
                _new_size: *mut ViewRect,
            ) -> tresult {
                kResultFalse
            }
        }

        let view = view_ptr();
        let frame_wrapper = ComWrapper::new(TestFrame);
        let frame = frame_wrapper
            .to_com_ptr::<IPlugFrame>()
            .expect("IPlugFrame is implemented");
        // SAFETY: `frame.as_ptr()` is a live COM pointer; the view is
        // live. `setFrame` addRefs — `frame` keeps its own reference.
        unsafe {
            assert_eq!(view.setFrame(ptr::null_mut()), kResultOk);
            assert_eq!(view.setFrame(frame.as_ptr()), kResultOk);
            assert_eq!(view.setFrame(ptr::null_mut()), kResultOk);
        }
    }

    /// The view-type constant the host passes to `createView` is the
    /// `"editor"` char8 string — the same bytes `cstring_eq` compares.
    #[expect(unsafe_code)]
    #[test]
    fn editor_view_type_constant_is_editor() {
        // SAFETY: `ViewType::kEditor` is a static NUL-terminated
        // char8 literal per the binding.
        unsafe {
            assert!(cstring_eq(ViewType::kEditor, "editor"));
            assert!(!cstring_eq(ViewType::kEditor, "other"));
        }
    }

    /// Windows-only: `attached` against a real parent HWND creates a
    /// real child window — the opt-in evidence path the #631
    /// real-dock lane formalizes. `#[ignore]` keeps it out of the
    /// default lane (it needs a window station); run explicitly with
    /// `cargo test -p ruviform-plugin-test-vst3 -- --ignored`.
    #[cfg(target_os = "windows")]
    #[ignore = "requires a Windows window station — #631 real-dock lane owns the formal evidence"]
    #[expect(unsafe_code)]
    #[test]
    fn attached_creates_real_child_hwnd() {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GW_CHILD, GetParent, GetWindow, IsWindow, WS_OVERLAPPED,
        };

        // The built-in STATIC class needs no registration — the
        // parent exists only to own the child.
        let class: Vec<u16> = "STATIC".encode_utf16().chain(std::iter::once(0)).collect();
        // A hidden top-level parent — no message pump needed for
        // attach verification.
        let parent = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                ptr::null(),
                WS_OVERLAPPED,
                0,
                0,
                320,
                240,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null(),
            )
        };
        assert!(!parent.is_null(), "cannot create a parent HWND");

        let view = view_ptr();
        let hwnd_type = vst3::Steinberg::kPlatformTypeHWND;
        // SAFETY: `parent` is a live HWND owned by this thread;
        // `view` is live.
        unsafe {
            assert_eq!(view.attached(parent, hwnd_type), kResultOk);
            // The child is observable through Win32 directly: the
            // parent must now have exactly one child, parented
            // correctly and alive.
            let child: HWND = GetWindow(parent, GW_CHILD);
            assert!(!child.is_null(), "attached must create a child HWND");
            assert_eq!(GetParent(child), parent, "child must be parented");
            assert_ne!(IsWindow(child), 0, "child must be a live window");
            // A second `attached` without `removed` is a defined
            // rejection, not a second window.
            assert_eq!(view.attached(parent, hwnd_type), kResultFalse);
            assert_eq!(view.removed(), kResultOk);
            assert_eq!(IsWindow(child), 0, "removed must destroy the child");
            let _ = DestroyWindow(parent);
        }
    }
}
