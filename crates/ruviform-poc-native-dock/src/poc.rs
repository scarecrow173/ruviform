//! The `PoC` driver: real GPUI window + real Win32 surface container +
//! real native probe editor, wired through the RF388/RF389 contracts.
//!
//! All evidence lines use the `[poc] key=value` form so the run log is
//! the evidence artifact itself.

mod dock;
mod dock_windowing;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, GuiHandle, ParamInfo, ParentLease, PluginDescriptor,
    PrepareError, ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo,
};
use ruviform_plugin_host::{
    AttachMode, EditorOpError, EditorSize, HostCommand, HostError, HostReply, InProcessEditorOps,
    InProcessHost, ParentSlotLease, Plugin, PluginEditorSessionId, PluginEditorSessions,
    PluginFactory, PluginHandle, erase,
};
use ruviform_ui::{
    DriverEvent, LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotError, SlotGeometry,
    SlotLease, SlotSpec, SlotVisibility, Support, SurfaceSlotId, ViewInstanceId, WindowDriver,
    WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::{GpuiDriver, GpuiSurfaceAdapter};

const PROBE_ID: &str = "org.ruviform.poc.native-editor-probe";
const SIZE: EditorSize = EditorSize {
    width: 400.0,
    height: 300.0,
};

// ---- probe plugin: a real native child-HWND editor -------------------

/// Shared observation channel between the probe and the harness.
struct ProbeShared {
    /// The redeemed payload the redeemer handed `open_gui`.
    parent_token: Cell<u64>,
    /// The real child HWND the editor created.
    child_hwnd: Cell<usize>,
    /// `open_gui` calls observed.
    opens: Cell<u32>,
    /// `close_gui` calls observed.
    closes: Cell<u32>,
}

/// `open_gui` creates a real `WS_CHILD` window under the redeemed
/// parent token — the same OS path a plugin binary takes.
#[expect(unsafe_code)]
mod probe {
    use ruviform_plugin_host::HostError;
    use windows_sys::Win32::Foundation::{HWND, LRESULT, RECT};
    use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
        RegisterClassW, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
    };

    /// Solid-fill editor color (pure red) so the attached surface is
    /// visually distinguishable inside the GPUI window.
    const EDITOR_BRUSH: u32 = 0x00_00_00_FF;

    /// Message counters observed by the probe editor's wnd_proc —
    /// deterministic key/focus delivery evidence for the in-process
    /// probe (the external-VST3 path relies on the `WH_GETMESSAGE`
    /// hook instead).
    pub(super) struct ProbeMessages {
        pub(super) keydown: std::sync::atomic::AtomicUsize,
        pub(super) char: std::sync::atomic::AtomicUsize,
        pub(super) setfocus: std::sync::atomic::AtomicUsize,
        pub(super) paint: std::sync::atomic::AtomicUsize,
        pub(super) erase: std::sync::atomic::AtomicUsize,
    }

    /// Process-wide probe message counters — one probe editor exists
    /// per PoC run, so statics are sufficient.
    pub(super) static PROBE_MESSAGES: ProbeMessages = ProbeMessages {
        keydown: std::sync::atomic::AtomicUsize::new(0),
        char: std::sync::atomic::AtomicUsize::new(0),
        setfocus: std::sync::atomic::AtomicUsize::new(0),
        paint: std::sync::atomic::AtomicUsize::new(0),
        erase: std::sync::atomic::AtomicUsize::new(0),
    };

    /// The class background brush handle, kept for `WM_PAINT` —
    /// `DefWindowProc` alone never erases the background because it
    /// does not call `BeginPaint`, so without an explicit paint the
    /// editor surface stays unpainted and the parent's pixels show
    /// through (observed as GPUI `control_track` in screen captures).
    static EDITOR_BRUSH_H: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

    /// `(keydown, char, setfocus, paint, erase)` counts so far.
    pub(super) fn probe_message_counts() -> (usize, usize, usize, usize, usize) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            PROBE_MESSAGES.keydown.load(Relaxed),
            PROBE_MESSAGES.char.load(Relaxed),
            PROBE_MESSAGES.setfocus.load(Relaxed),
            PROBE_MESSAGES.paint.load(Relaxed),
            PROBE_MESSAGES.erase.load(Relaxed),
        )
    }

    /// Window proc counting keyboard/focus messages as delivery
    /// evidence and painting the solid editor brush itself — the
    /// class brush alone never reaches the screen because
    /// `DefWindowProc`'s `WM_PAINT` path skips `BeginPaint`, so the
    /// erase that would apply `hbrBackground` never runs.
    ///
    /// # Safety
    /// Called by the OS message loop only, on our own HWND.
    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> LRESULT {
        use std::sync::atomic::Ordering::Relaxed;
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::Graphics::Gdi::{
            BeginPaint, EndPaint, FillRect, HBRUSH, PAINTSTRUCT,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WM_CHAR, WM_ERASEBKGND, WM_KEYDOWN, WM_PAINT, WM_SETFOCUS,
        };
        match msg {
            WM_KEYDOWN => {
                PROBE_MESSAGES.keydown.fetch_add(1, Relaxed);
            }
            WM_CHAR => {
                PROBE_MESSAGES.char.fetch_add(1, Relaxed);
            }
            WM_SETFOCUS => {
                PROBE_MESSAGES.setfocus.fetch_add(1, Relaxed);
            }
            WM_ERASEBKGND => {
                PROBE_MESSAGES.erase.fetch_add(1, Relaxed);
                // SAFETY: `wparam` is the erase-target HDC and
                // `lparam` a `*const RECT` update rect supplied by
                // the system for this call.
                unsafe {
                    let brush = EDITOR_BRUSH_H.load(Relaxed) as HBRUSH;
                    FillRect(wparam as _, lparam as *const RECT, brush);
                }
                return 1;
            }
            WM_PAINT => {
                PROBE_MESSAGES.paint.fetch_add(1, Relaxed);
                // SAFETY: `BeginPaint`/`EndPaint` bracket the paint
                // DC for `hwnd`; `GetClientRect` fills `rc`.
                unsafe {
                    let mut ps = PAINTSTRUCT::default();
                    let hdc = BeginPaint(hwnd, &raw mut ps);
                    let mut rc = RECT::default();
                    GetClientRect(hwnd, &raw mut rc);
                    let brush = EDITOR_BRUSH_H.load(Relaxed) as HBRUSH;
                    FillRect(hdc, &raw const rc, brush);
                    EndPaint(hwnd, &raw const ps);
                }
                return 0;
            }
            _ => {}
        }
        // SAFETY: forwards every message to the default handler.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    /// Registers the probe editor class exactly once.
    fn probe_class() -> Vec<u16> {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let name: Vec<u16> = "RvfProbeEditor"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: the class name outlives the registration via
            // `mem::forget` (Windows does not copy `lpszClassName`);
            // the brush intentionally leaks — it must outlive every
            // editor window the class creates.
            unsafe {
                let brush = CreateSolidBrush(EDITOR_BRUSH);
                EDITOR_BRUSH_H.store(brush as isize, std::sync::atomic::Ordering::Relaxed);
                let class = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: GetModuleHandleW(core::ptr::null()),
                    hIcon: core::ptr::null_mut(),
                    hCursor: core::ptr::null_mut(),
                    hbrBackground: brush,
                    lpszMenuName: core::ptr::null(),
                    lpszClassName: name.as_ptr(),
                };
                let atom = RegisterClassW(&raw const class);
                assert!(atom != 0, "RegisterClassW failed");
            }
            std::mem::forget(name);
        });
        "RvfProbeEditor"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Creates the editor child window under `parent` (the container
    /// HWND), filling the parent's client area.
    ///
    /// # Errors
    /// [`HostError::PluginFailed`] when `CreateWindowExW` fails.
    pub(super) fn create_editor(
        parent: u64,
        shared: &super::ProbeShared,
    ) -> Result<usize, HostError> {
        let class = probe_class();
        let parent_hwnd = usize::try_from(parent).map_err(|_| HostError::PluginFailed)? as HWND;
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `parent_hwnd` is the live container HWND the adapter
        // redeemed; GetClientRect writes `rc`.
        unsafe {
            GetClientRect(parent_hwnd, &raw mut rc);
        }
        // SAFETY: parenting under the redeemed HWND is the defined
        // contract; the class was registered above; the HWND is ours
        // until close_gui destroys it.
        let child = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                core::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                parent_hwnd,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null(),
            )
        };
        if child.is_null() {
            return Err(HostError::PluginFailed);
        }
        shared.child_hwnd.set(child as usize);
        Ok(child as usize)
    }

    /// Diagnostic sibling: a second probe-class `WS_CHILD` window
    /// under `parent` at `(x, y, w, h)` — does NOT touch
    /// `shared.child_hwnd`. Used to test whether *any* child of the
    /// container composites, independent of the plugin editor.
    ///
    /// # Errors
    /// [`HostError::PluginFailed`] when `CreateWindowExW` fails.
    pub(super) fn create_probe_child(
        parent: u64,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<usize, HostError> {
        let class = probe_class();
        let parent_hwnd = usize::try_from(parent).map_err(|_| HostError::PluginFailed)? as HWND;
        // SAFETY: same contract as `create_editor` — the parent HWND
        // is live and owned by this thread.
        let child = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                core::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                x,
                y,
                w,
                h,
                parent_hwnd,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null(),
            )
        };
        if child.is_null() {
            return Err(HostError::PluginFailed);
        }
        Ok(child as usize)
    }

    /// Destroys the editor child window.
    pub(super) fn destroy_editor(hwnd: usize) {
        // SAFETY: `hwnd` is the child HWND `create_editor` returned;
        // DestroyWindow on an already-dead handle is a harmless error.
        unsafe {
            DestroyWindow(hwnd as HWND);
        }
    }
}

struct NativeEditorProbe {
    shared: Rc<ProbeShared>,
}

impl ruviform_plugin_core::PluginProcessor for NativeEditorProbe {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        _inputs: &[I],
        _outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
    }
}

impl Plugin for NativeEditorProbe {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: PROBE_ID,
            name: "Native Editor Probe",
            vendor: "Ruviform",
            version: "0.1.0",
            features: &[],
        }
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn open_gui(&mut self, parent: ParentLease) -> Result<GuiHandle, HostError> {
        self.shared.parent_token.set(parent.payload());
        self.shared.opens.set(self.shared.opens.get() + 1);
        let _child = probe::create_editor(parent.payload(), &self.shared)?;
        Ok(GuiHandle::new(1))
    }

    fn close_gui(&mut self) -> Result<(), HostError> {
        let hwnd = self.shared.child_hwnd.get();
        if hwnd != 0 {
            probe::destroy_editor(hwnd);
            self.shared.child_hwnd.set(0);
        }
        self.shared.closes.set(self.shared.closes.get() + 1);
        Ok(())
    }
}

struct ProbeFactory {
    shared: Rc<ProbeShared>,
}

impl PluginFactory for ProbeFactory {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: PROBE_ID,
            name: "Native Editor Probe",
            vendor: "Ruviform",
            version: "0.1.0",
            features: &[],
        }
    }

    fn create(&self) -> Box<dyn ruviform_plugin_host::ErasedPlugin> {
        erase(NativeEditorProbe {
            shared: self.shared.clone(),
        })
    }
}

// ---- OS observation helpers (evidence probes on real HWNDs) -----------

/// `(x, y, w, h)` of `hwnd` in screen coordinates, if it exists.
#[expect(unsafe_code)]
fn window_rect(hwnd: usize) -> Option<(i32, i32, i32, i32)> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsWindow};
    // SAFETY: queries only; `IsWindow` guards dead handles.
    unsafe {
        if IsWindow(hwnd as HWND) == 0 {
            return None;
        }
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(hwnd as HWND, &raw mut rc);
        Some((rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top))
    }
}

/// Whether `hwnd` exists and is visible.
#[expect(unsafe_code)]
fn is_visible(hwnd: usize) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, IsWindowVisible};
    // SAFETY: queries only.
    unsafe { IsWindow(hwnd as HWND) != 0 && IsWindowVisible(hwnd as HWND) != 0 }
}

/// Whether `hwnd` still exists.
#[expect(unsafe_code)]
fn is_window(hwnd: usize) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
    // SAFETY: query only.
    unsafe { IsWindow(hwnd as HWND) != 0 }
}

/// `GetParent(child)` — proves real parenting under the container.
#[expect(unsafe_code)]
fn parent_of(hwnd: usize) -> usize {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetParent;
    // SAFETY: query only.
    unsafe { GetParent(hwnd as HWND) as usize }
}

/// One-line description of an ancestor HWND — class name, window
/// rect, window region, client-DC clip box, and style/exstyle — used
/// to find which ancestor empties a descendant's visible region.
#[expect(unsafe_code)]
fn ancestor_line(hwnd: usize, depth: usize) -> String {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetWindowLongPtrW, GetWindowRect,
    };
    if hwnd == 0 {
        return format!("{depth}: <null>");
    }
    // SAFETY: queries only on a live-or-dead handle; dead handles
    // yield zeroed fields, which is itself diagnostic.
    unsafe {
        let mut name = [0u16; 64];
        let n = GetClassNameW(hwnd as HWND, name.as_mut_ptr(), name.len() as i32);
        let cls = if n > 0 {
            String::from_utf16_lossy(&name[..n as usize])
        } else {
            "?".to_string()
        };
        let mut rc = RECT::default();
        GetWindowRect(hwnd as HWND, &raw mut rc);
        let style = GetWindowLongPtrW(hwnd as HWND, GWL_STYLE) as u32;
        let exstyle = GetWindowLongPtrW(hwnd as HWND, GWL_EXSTYLE) as u32;
        format!(
            "{depth}: hwnd=0x{hwnd:x} cls={cls} rect=({},{},{},{}) style=0x{style:08x} \
             exstyle=0x{exstyle:08x} region={:?} clip={:?}",
            rc.left,
            rc.top,
            rc.right - rc.left,
            rc.bottom - rc.top,
            region_box(hwnd),
            clip_box(hwnd),
        )
    }
}

/// `child` plus every `GetParent` ancestor up to the desktop root —
/// one [`ancestor_line`] per level.
fn ancestor_chain(child: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = child;
    for depth in 0..8 {
        out.push(ancestor_line(cur, depth));
        let parent = parent_of(cur);
        if parent == 0 || parent == cur {
            break;
        }
        cur = parent;
    }
    out
}

/// The first child HWND of `parent` — how the harness discovers the
/// editor window a real external plugin created under the container
/// (the in-process probe reports its own HWND; a third-party binary
/// does not, so we observe the OS truth directly).
#[expect(unsafe_code)]
fn first_child_of(parent: u64) -> usize {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows;
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        // SAFETY: `lparam` is the `*mut Vec<usize>` the caller passed;
        // it outlives this synchronous enumeration.
        unsafe { (*(lparam as *mut Vec<usize>)).push(hwnd as usize) };
        1
    }
    let mut found: Vec<usize> = Vec::new();
    // SAFETY: query-only enumeration; `found` outlives the call.
    unsafe {
        EnumChildWindows(parent as HWND, Some(enum_proc), &raw mut found as LPARAM);
    }
    found.first().copied().unwrap_or(0)
}

/// The HWND holding keyboard focus.
#[expect(unsafe_code)]
fn focus_hwnd() -> usize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    // SAFETY: query only.
    unsafe { GetFocus() as usize }
}

/// `(w, h)` of `hwnd`'s client area in physical pixels.
#[expect(unsafe_code)]
fn client_size(hwnd: usize) -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, IsWindow};
    // SAFETY: queries only; `IsWindow` guards dead handles.
    unsafe {
        if IsWindow(hwnd as HWND) == 0 {
            return None;
        }
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd as HWND, &raw mut rc);
        Some((rc.right - rc.left, rc.bottom - rc.top))
    }
}

/// `(left, top, right, bottom)` of `hwnd`'s window region in window
/// coordinates, or `None` when no region is set — the observable
/// result of the `SetWindowRgn` clip path on the slot container.
/// Same pattern as the adapter's own test helper: `GetWindowRgn`
/// copies the window's region into a caller-owned probe HRGN.
#[expect(unsafe_code)]
fn region_box(hwnd: usize) -> Option<(i32, i32, i32, i32)> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetRgnBox, GetWindowRgn};
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
    // SAFETY: queries only; `IsWindow` guards dead handles. `probe`
    // is an owned HRGN the callee only copies into.
    unsafe {
        if IsWindow(hwnd as HWND) == 0 {
            return None;
        }
        let probe = CreateRectRgn(0, 0, 0, 0);
        if probe.is_null() {
            return None;
        }
        let kind = GetWindowRgn(hwnd as HWND, probe);
        if kind <= 1 {
            // ERROR or NULLREGION — no explicit region attached.
            DeleteObject(probe);
            return None;
        }
        let mut r = RECT::default();
        let ok = GetRgnBox(probe, &raw mut r);
        DeleteObject(probe);
        (ok != 0).then_some((r.left, r.top, r.right, r.bottom))
    }
}

/// Every descendant HWND of `root` — `EnumChildWindows` already
/// walks the tree recursively, so one call collects the whole
/// subtree below `root`.
#[expect(unsafe_code)]
fn descendants_of(root: usize) -> Vec<usize> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows;
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        // SAFETY: `lparam` is the `*mut Vec<usize>` the caller passed;
        // it outlives this synchronous enumeration.
        unsafe { (*(lparam as *mut Vec<usize>)).push(hwnd as usize) };
        1
    }
    let mut found: Vec<usize> = Vec::new();
    // SAFETY: query-only enumeration; `found` outlives the call.
    unsafe {
        EnumChildWindows(root as HWND, Some(enum_proc), &raw mut found as LPARAM);
    }
    found
}

/// `root` plus every descendant — the "plugin window tree" used to
/// attribute delivered messages and keyboard focus to the plugin
/// side of the container boundary.
fn window_tree(root: usize) -> Vec<usize> {
    let mut tree = vec![root];
    tree.extend(descendants_of(root));
    tree
}

/// `(left, top, right, bottom)` of the clip region on `hwnd`'s own
/// client DC — the region `GetPixel` can actually read. An empty or
/// degenerate box means the window's visible region excludes the
/// sample points even when `GetWindowRect` reports a sane rect.
#[expect(unsafe_code)]
fn clip_box(hwnd: usize) -> Option<(i32, i32, i32, i32)> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{GetClipBox, GetDC, ReleaseDC};
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
    // SAFETY: queries only; `IsWindow` guards dead handles and the DC
    // is released before returning.
    unsafe {
        if IsWindow(hwnd as HWND) == 0 {
            return None;
        }
        let dc = GetDC(hwnd as HWND);
        if dc.is_null() {
            return None;
        }
        let mut rc = RECT::default();
        let kind = GetClipBox(dc, &raw mut rc);
        ReleaseDC(hwnd as HWND, dc);
        (kind != 0).then_some((rc.left, rc.top, rc.right, rc.bottom))
    }
}

/// `(hwnd, message)` pairs retrieved by this thread's message queue,
/// recorded by the `WH_GETMESSAGE` hook — which window each injected
/// key was actually delivered to.
static MSG_LOG: std::sync::Mutex<Vec<(usize, u32)>> = std::sync::Mutex::new(Vec::new());
static MSG_HOOK: std::sync::Mutex<usize> = std::sync::Mutex::new(0);

/// `WH_GETMESSAGE` hook proc — appends `(hwnd, message)` for every
/// message the hooked thread retrieves, then chains on.
#[expect(unsafe_code)]
unsafe extern "system" fn getmsg_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CallNextHookEx, HC_ACTION, MSG};
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_GETMESSAGE, `lparam` is a `*const MSG`
        // provided by the system for the duration of the call.
        let msg = unsafe { &*(lparam as *const MSG) };
        MSG_LOG
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((msg.hwnd as usize, msg.message));
    }
    // SAFETY: chaining with a null hook handle is the documented
    // pass-through for hook procs.
    unsafe { CallNextHookEx(core::ptr::null_mut(), code, wparam, lparam) }
}

/// Installs the message-recording hook on `thread_id`'s queue.
///
/// # Errors
/// `false` when `SetWindowsHookExW` refuses the hook.
#[expect(unsafe_code)]
fn hook_messages(thread_id: u32) -> bool {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_GETMESSAGE};
    // SAFETY: `getmsg_proc` matches the `HOOKPROC` ABI; the module
    // handle is only needed for global hooks but is harmless here.
    let hook = unsafe {
        SetWindowsHookExW(
            WH_GETMESSAGE,
            Some(getmsg_proc),
            GetModuleHandleW(core::ptr::null()),
            thread_id,
        )
    };
    if hook.is_null() {
        return false;
    }
    *MSG_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = hook as usize;
    true
}

/// Removes the message-recording hook installed by
/// [`hook_messages`].
#[expect(unsafe_code)]
fn unhook_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{HHOOK, UnhookWindowsHookEx};
    let hook = *MSG_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if hook != 0 {
        // SAFETY: `hook` is the live handle `hook_messages` stored.
        unsafe {
            UnhookWindowsHookEx(hook as HHOOK);
        }
        *MSG_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = 0;
    }
}

/// Drains the recorded `(hwnd, message)` log.
fn drain_msg_log() -> Vec<(usize, u32)> {
    MSG_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect()
}

/// The OS thread that owns `hwnd` — used to scope the
/// `WH_GETMESSAGE` hook to our own queue.
#[expect(unsafe_code)]
fn window_thread_id(hwnd: usize) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    // SAFETY: query only; a null process-id out-param is allowed.
    unsafe { GetWindowThreadProcessId(hwnd as _, core::ptr::null_mut()) }
}

/// Injects a real key press+release for virtual-key code `vk`.
#[expect(unsafe_code)]
fn send_key(vk: u8) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_KEYUP, keybd_event};
    // SAFETY: `keybd_event` injects trusted input into the focused
    // window of the active desktop — the PoC's defined evidence path.
    unsafe {
        keybd_event(vk, 0, 0, 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

/// Moves the cursor to screen point `(x, y)` and clicks the left
/// button — the real input path a user takes to focus a plugin
/// editor.
#[expect(unsafe_code)]
fn click_at(x: i32, y: i32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, mouse_event,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;
    // SAFETY: synthetic input on the PoC's own window; coordinates
    // come from `GetWindowRect` of the editor.
    unsafe {
        SetCursorPos(x, y);
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
}

/// `SetFocus` on `hwnd`, returning the previously focused HWND.
#[expect(unsafe_code)]
fn set_focus(hwnd: usize) -> usize {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    // SAFETY: `hwnd` is a live window owned by this thread.
    unsafe { SetFocus(hwnd as HWND) as usize }
}

/// COLORREF of the screen pixel at `(x, y)` — `0xFFFF_FFFF`
/// (`CLR_INVALID`) on failure. Used for clip-boundary pixel checks.
#[expect(unsafe_code)]
fn screen_pixel(x: i32, y: i32) -> u32 {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
    // SAFETY: `GetDC(NULL)` returns the screen DC; `GetPixel`/`ReleaseDC`
    // are paired correctly and the DC is not retained.
    unsafe {
        let dc = GetDC(core::ptr::null_mut());
        let px = GetPixel(dc, x, y);
        ReleaseDC(core::ptr::null_mut(), dc);
        px
    }
}

/// `(COLORREF, dc_was_null)` of `hwnd`'s own client-area pixel at
/// `(x, y)` — read through the window's own DC, so it reflects what
/// the window painted regardless of screen composition or overlapping
/// windows. The pixel is `0xFFFF_FFFF` (`CLR_INVALID`) on failure.
#[expect(unsafe_code)]
fn window_pixel(hwnd: usize, x: i32, y: i32) -> (u32, bool) {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
    // SAFETY: `GetDC(hwnd)` returns the window's client DC;
    // `GetPixel`/`ReleaseDC` are paired and the DC is not retained.
    unsafe {
        let dc = GetDC(hwnd as _);
        if dc.is_null() {
            return (0xFFFF_FFFF, true);
        }
        let px = GetPixel(dc, x, y);
        ReleaseDC(hwnd as _, dc);
        (px, false)
    }
}

/// The topmost window under screen point `(x, y)` — `WindowFromPoint`
/// returns the deepest child, so a live plugin editor answers with
/// its own HWND (or an intermediate child), never the GPUI frame.
#[expect(unsafe_code)]
fn window_from_point(x: i32, y: i32) -> usize {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint;
    // SAFETY: query only.
    unsafe { WindowFromPoint(POINT { x, y }) as usize }
}

/// COLORREF of `hwnd`'s center pixel as rendered by `PrintWindow`
/// (`PW_RENDERFULLCONTENT`) into a memory DC — asks the window to
/// paint itself, bypassing screen composition and overlapping
/// windows entirely. `0xFFFF_FFFF` on any failure.
#[expect(unsafe_code)]
fn printwindow_pixel(hwnd: usize) -> u32 {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetPixel,
        ReleaseDC, SelectObject,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, IsWindow, PW_RENDERFULLCONTENT,
    };
    // SAFETY: all GDI objects are created/selected/deleted in pairs;
    // `IsWindow` guards the handle before use.
    unsafe {
        if IsWindow(hwnd as _) == 0 {
            return 0xFFFF_FFFF;
        }
        let mut rc = RECT::default();
        GetWindowRect(hwnd as _, &raw mut rc);
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        if w <= 0 || h <= 0 {
            return 0xFFFF_FFFF;
        }
        let screen = GetDC(core::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp);
        let printed = PrintWindow(hwnd as _, mem, PW_RENDERFULLCONTENT);
        let px = if printed != 0 {
            GetPixel(mem, w / 2, h / 2)
        } else {
            0xFFFF_FFFE
        };
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(core::ptr::null_mut(), screen);
        px
    }
}

/// Emits one `[poc] key=value` evidence line.
fn ev(line: &str) {
    eprintln!("[poc] {line}");
}

/// Builds the session ops with the consumer-side redeemer — the
/// composition the architecture baseline reserves for this crate:
/// the adapter's live-lease validation turns an opaque
/// [`ParentSlotLease`] payload into the generation-bound
/// [`ParentLease`] `open_gui` consumes.
fn make_ops(
    host: &mut InProcessHost,
    adapter: Rc<RefCell<GpuiSurfaceAdapter>>,
    instance: PluginHandle,
) -> InProcessEditorOps<'_> {
    InProcessEditorOps::with_redeemer(
        host,
        instance,
        Box::new(move |lease: &ParentSlotLease| {
            adapter
                .borrow()
                .redeem(lease.raw())
                .map(|token| ParentLease::new(SlotLease::new(lease.raw()).seq(), token))
                .map_err(|error| match error {
                    SlotError::Unsupported => EditorOpError::Unsupported,
                    _ => EditorOpError::LeaseRejected,
                })
        }),
    )
}

/// Registers the plugin half and returns `(plugin_id, vst3_mode)`.
///
/// With `--features vst3` and `RVF_POC_VST3_PATH` set, loads the real
/// external VST3 module through the `host` bridge and registers every
/// scanned class — the in-process probe stays the deterministic
/// default for every other configuration. `RVF_POC_VST3_PLUGIN_ID`
/// selects one class when the module ships several; otherwise the
/// first scanned descriptor is driven.
#[cfg(feature = "vst3")]
fn register_plugin(host: &mut InProcessHost, shared: &Rc<ProbeShared>) -> (String, bool) {
    use ruviform_plugin_vst3::host::Vst3Library;
    if let Some(path) = std::env::var_os("RVF_POC_VST3_PATH") {
        let path = std::path::PathBuf::from(path);
        let size = std::fs::metadata(&path).map_or(0, |m| m.len());
        ev(&format!("vst3_path={}", path.display()));
        ev(&format!("vst3_size={size}"));
        match Vst3Library::load(&path) {
            Ok(library) => {
                let descriptors = library.descriptors().to_vec();
                ev(&format!("vst3_descriptors={}", descriptors.len()));
                for factory in library.factories() {
                    host.register_factory(Box::new(factory));
                }
                let wanted = std::env::var("RVF_POC_VST3_PLUGIN_ID").ok();
                let id = match wanted {
                    Some(w) if descriptors.iter().any(|d| d.id == w) => w,
                    _ => descriptors
                        .first()
                        .map(|d| d.id.to_string())
                        .unwrap_or_default(),
                };
                if let Some(d) = descriptors.iter().find(|d| d.id == id) {
                    ev(&format!(
                        "vst3_plugin id={} name={} vendor={} version={}",
                        d.id, d.name, d.vendor, d.version
                    ));
                }
                return (id, true);
            }
            Err(e) => {
                ev(&format!("vst3_load=fail error={e:?}"));
                std::process::exit(3);
            }
        }
    }
    ev("vst3_path=unset mode=probe");
    host.register_factory(Box::new(ProbeFactory {
        shared: shared.clone(),
    }));
    (PROBE_ID.to_string(), false)
}

/// Probe-only registration for non-`vst3` builds.
#[cfg(not(feature = "vst3"))]
fn register_plugin(host: &mut InProcessHost, shared: &Rc<ProbeShared>) -> (String, bool) {
    host.register_factory(Box::new(ProbeFactory {
        shared: shared.clone(),
    }));
    (PROBE_ID.to_string(), false)
}

// ---- the PoC state machine -------------------------------------------

/// Ordered stages; each stage counter settles a fixed number of ticks
/// before the next action so OS-level effects flush through the
/// message queue. `Copy` keeps the driver free of borrow juggling.
#[derive(Copy, Clone, Debug)]
enum Stage {
    WaitReady,
    Attached { settle: u32 },
    Moved { settle: u32 },
    Hidden { settle: u32 },
    Restored { settle: u32 },
    Closing { settle: u32 },
    Done,
}

struct Poc {
    window: WindowId,
    adapter: Rc<RefCell<GpuiSurfaceAdapter>>,
    host: InProcessHost,
    sessions: PluginEditorSessions,
    session: Option<PluginEditorSessionId>,
    instance: PluginHandle,
    slot: Option<SurfaceSlotId>,
    slot_lease: Option<SlotLease>,
    probe: Rc<ProbeShared>,
    /// Real-external-VST3 mode — the editor HWND is discovered via
    /// `EnumChildWindows` instead of plugin-reported.
    vst3_mode: bool,
    stage: Stage,
    /// Child rect snapshot before the move (for delta verification).
    rect_before_move: Option<(i32, i32, i32, i32)>,
}

impl Poc {
    /// Ready → declare slot, lease parent, open the editor session.
    fn attach(&mut self, drv: &mut dyn WindowDriver) {
        let scale = drv.monitors().first().map_or(1.0, |m| m.scale_factor);
        let caps = SlotCapabilities {
            attach_parent: Support::Supported,
            hide_show: Support::Supported,
            focus: Support::Supported,
            clip: Support::Supported,
            z_order: Support::Supported,
            ..SlotCapabilities::default()
        };
        let spec = SlotSpec {
            window: self.window,
            view: ViewInstanceId::new(1).expect("nonzero"),
            geometry: SlotGeometry {
                rect: LogicalRect {
                    x: 40.0,
                    y: 40.0,
                    width: SIZE.width,
                    height: SIZE.height,
                },
                scale_factor: scale,
            },
            visibility: SlotVisibility::Visible,
            capabilities: caps,
        };
        let slot = match self.adapter.borrow_mut().declare_slot(spec) {
            Ok(slot) => slot,
            Err(e) => {
                ev(&format!("declare_slot=fail error={e}"));
                self.stage = Stage::Done;
                return;
            }
        };
        self.slot = Some(slot);
        ev(&format!("declare_slot=ok index={}", slot.index()));

        let lease = match self.adapter.borrow_mut().lease_parent(slot) {
            Ok(lease) => lease,
            Err(e) => {
                ev(&format!("lease_parent=fail error={e}"));
                self.stage = Stage::Done;
                return;
            }
        };
        self.slot_lease = Some(lease);
        let container = self.adapter.borrow().container_token(slot).unwrap_or(0);
        ev(&format!("lease_parent=ok container_hwnd=0x{container:x}"));

        let parent = ParentSlotLease::new(lease.raw());
        let open_result = {
            let mut ops = make_ops(&mut self.host, self.adapter.clone(), self.instance);
            let session = self.session.expect("session begun");
            self.sessions
                .session_mut(session)
                .expect("session live")
                .open(&mut ops, &parent, SIZE, AttachMode::Docked)
        };
        match open_result {
            Ok(caps) => {
                if self.vst3_mode {
                    // The external plugin does not report its editor
                    // HWND — observe the container's children directly
                    // and pin the parent token for the parenting check.
                    self.probe.parent_token.set(container);
                    self.probe.child_hwnd.set(first_child_of(container));
                }
                ev(&format!(
                    "open=ok caps_dock={caps_dock:?} caps_close={caps_close:?}",
                    caps_dock = caps.dock,
                    caps_close = caps.close
                ));
                ev(&format!(
                    "editor_hwnd=0x{:x} parent_token=0x{:x} opens={}",
                    self.probe.child_hwnd.get(),
                    self.probe.parent_token.get(),
                    self.probe.opens.get()
                ));
                self.rect_before_move = window_rect(self.probe.child_hwnd.get());
                ev(&format!(
                    "editor_rect={:?} parenting_ok={} editor_visible={}",
                    self.rect_before_move,
                    parent_of(self.probe.child_hwnd.get())
                        == usize::try_from(self.probe.parent_token.get()).unwrap_or(0),
                    is_visible(self.probe.child_hwnd.get())
                ));
                self.stage = Stage::Attached { settle: 0 };
            }
            Err(e) => {
                ev(&format!("open=fail error={e:?}"));
                self.stage = Stage::Done;
            }
        }
    }

    /// `Attached` → move the slot and observe the editor's rect follow.
    fn do_move(&mut self, drv: &mut dyn WindowDriver) {
        let slot = self.slot.expect("slot");
        let scale = drv.monitors().first().map_or(1.0, |m| m.scale_factor);
        let result = self.adapter.borrow_mut().set_geometry(
            slot,
            SlotGeometry {
                rect: LogicalRect {
                    x: 80.0,
                    y: 90.0,
                    width: SIZE.width,
                    height: SIZE.height,
                },
                scale_factor: scale,
            },
        );
        let after = window_rect(self.probe.child_hwnd.get());
        ev(&format!(
            "set_geometry={:?} rect_before={:?} rect_after={:?}",
            result.is_ok(),
            self.rect_before_move,
            after
        ));
        self.stage = Stage::Moved { settle: 0 };
    }

    /// `Moved` → hide the slot and observe the editor disappear.
    fn do_hide(&mut self) {
        let slot = self.slot.expect("slot");
        let result = self
            .adapter
            .borrow_mut()
            .set_visibility(slot, SlotVisibility::Hidden);
        ev(&format!(
            "hide={:?} child_visible={}",
            result.is_ok(),
            is_visible(self.probe.child_hwnd.get())
        ));
        self.stage = Stage::Hidden { settle: 0 };
    }

    /// `Hidden` → re-show the slot and focus the native surface.
    fn do_restore(&mut self) {
        let slot = self.slot.expect("slot");
        let result = self
            .adapter
            .borrow_mut()
            .set_visibility(slot, SlotVisibility::Visible);
        ev(&format!(
            "show={:?} child_visible={}",
            result.is_ok(),
            is_visible(self.probe.child_hwnd.get())
        ));
        let focus = self.adapter.borrow_mut().request_focus(slot);
        ev(&format!(
            "focus={:?} focus_hwnd=0x{:x}",
            focus.is_ok(),
            focus_hwnd()
        ));
        self.stage = Stage::Restored { settle: 0 };
    }

    /// `Restored` → close the session; `close_gui` destroys the editor.
    fn do_close(&mut self) {
        let session = self.session.expect("session begun");
        let close_result = {
            let mut ops = make_ops(&mut self.host, self.adapter.clone(), self.instance);
            self.sessions
                .session_mut(session)
                .expect("session live")
                .close(&mut ops)
        };
        let state = self.sessions.session_mut(session).map(|s| s.state());
        ev(&format!(
            "close={:?} state={state:?} closes={} child_alive={}",
            close_result.is_ok(),
            self.probe.closes.get(),
            is_window(self.probe.child_hwnd.get())
        ));
        self.stage = Stage::Closing { settle: 0 };
    }

    /// `Closing` → release the lease (container dies) and the slot,
    /// then drain the event queue for the evidence log.
    fn do_teardown(&mut self) {
        let slot = self.slot.expect("slot");
        let container = self.adapter.borrow().container_token(slot).unwrap_or(0);
        let lease = self.slot_lease.expect("lease minted");
        let released = self.adapter.borrow_mut().release_lease(lease);
        ev(&format!(
            "release_lease={:?} container_alive={}",
            released.is_ok(),
            is_window(usize::try_from(container).unwrap_or(0))
        ));
        let released_slot = self.adapter.borrow_mut().release_slot(slot);
        ev(&format!("release_slot={:?}", released_slot.is_ok()));
        for ev_line in self.adapter.borrow_mut().drain_events() {
            ev(&format!("slot_event={ev_line:?}"));
        }
        self.stage = Stage::Done;
    }

    /// Tick-advance: run the stage machine. Action bodies live in
    /// `do_*` methods; the settle counter throttles each stage so
    /// OS-level effects flush through the message queue first.
    fn tick(&mut self, drv: &mut dyn WindowDriver) {
        const SETTLE_TICKS: u32 = 20;
        match self.stage {
            Stage::Attached { settle } if settle >= SETTLE_TICKS => self.do_move(drv),
            Stage::Attached { settle } => {
                self.stage = Stage::Attached { settle: settle + 1 };
            }
            Stage::Moved { settle } if settle >= SETTLE_TICKS => self.do_hide(),
            Stage::Moved { settle } => {
                self.stage = Stage::Moved { settle: settle + 1 };
            }
            Stage::Hidden { settle } if settle >= SETTLE_TICKS => self.do_restore(),
            Stage::Hidden { settle } => {
                self.stage = Stage::Hidden { settle: settle + 1 };
            }
            Stage::Restored { settle } if settle >= SETTLE_TICKS => self.do_close(),
            Stage::Restored { settle } => {
                self.stage = Stage::Restored { settle: settle + 1 };
            }
            Stage::Closing { settle } if settle >= SETTLE_TICKS => self.do_teardown(),
            Stage::Closing { settle } => {
                self.stage = Stage::Closing { settle: settle + 1 };
            }
            Stage::Done => {
                let session = self.session.expect("session begun");
                let state = self.sessions.session_mut(session).map(|s| s.state());
                ev(&format!("done state={state:?}"));
                drv.exit();
            }
            Stage::WaitReady => {}
        }
    }
}

/// Registers the plugin, then instantiates and initializes one
/// instance. Returns `(instance, vst3_mode)`.
fn instantiate(host: &mut InProcessHost, shared: &Rc<ProbeShared>) -> (PluginHandle, bool) {
    let (plugin_id, vst3_mode) = register_plugin(host, shared);
    eprintln!("[poc] plugin_id={plugin_id} vst3_mode={vst3_mode}");
    let instance = match host.execute(HostCommand::Instantiate {
        plugin_id: plugin_id.clone(),
    }) {
        Ok(HostReply::Instantiated { handle }) => handle,
        other => panic!("instantiate failed: {other:?}"),
    };
    eprintln!("[poc] instance={instance:?}");
    // `open_gui` is only legal on an initialized instance — the
    // probe's default init is a no-op, so Initialize is uniform.
    match host.execute(HostCommand::Initialize { handle: instance }) {
        Ok(_) => eprintln!("[poc] initialize=ok"),
        Err(e) => eprintln!("[poc] initialize=fail error={e:?}"),
    }
    (instance, vst3_mode)
}

/// Entry point — builds the driver, window, host and session, then
/// runs the GPUI loop until the `PoC` finishes. `RVF_POC_SCENARIO=dock`
/// selects the Panel/Tab Dock layout scenario instead of the
/// floating-region probe.
pub fn run() {
    if std::env::var("RVF_POC_SCENARIO").is_ok_and(|s| s == "dock") {
        return dock::run();
    }
    if std::env::var("RVF_POC_SCENARIO").is_ok_and(|s| s == "dock-windowing") {
        return dock_windowing::run();
    }
    eprintln!("[poc] start os=windows");

    let mut driver = GpuiDriver::new();
    eprintln!("[poc] capabilities={:?}", driver.capabilities());

    let window = driver
        .create_window(&WindowSpec {
            title: "ruviform — native dock poc".to_string(),
            ..WindowSpec::default()
        })
        .expect("create_window");
    eprintln!("[poc] window={window}");

    let probe = Rc::new(ProbeShared {
        parent_token: Cell::new(0),
        child_hwnd: Cell::new(0),
        opens: Cell::new(0),
        closes: Cell::new(0),
    });
    let mut host = InProcessHost::new();
    let (instance, vst3_mode) = instantiate(&mut host, &probe);

    let mut sessions = PluginEditorSessions::new();
    let session = sessions.begin_session(instance).expect("begin_session");

    let mut poc = Poc {
        window,
        adapter: Rc::new(RefCell::new(GpuiSurfaceAdapter::new(&driver))),
        host,
        sessions,
        session: Some(session),
        instance,
        slot: None,
        slot_lease: None,
        probe,
        vst3_mode,
        stage: Stage::WaitReady,
        rect_before_move: None,
    };

    driver
        .run(None, move |ev, drv| match ev {
            DriverEvent::Windowed {
                event: WindowLifecycle::Ready,
                window: w,
            } if w == poc.window && matches!(poc.stage, Stage::WaitReady) => {
                eprintln!("[poc] ready");
                poc.attach(drv);
            }
            DriverEvent::Windowed {
                event: WindowLifecycle::CloseRequested,
                window: w,
            } if w == poc.window => {
                let _ = drv.respond_close(w, ruviform_ui::CloseDecision::Approve);
            }
            DriverEvent::Tick => poc.tick(drv),
            _ => {}
        })
        .expect("run");
    eprintln!("[poc] exited");
}
