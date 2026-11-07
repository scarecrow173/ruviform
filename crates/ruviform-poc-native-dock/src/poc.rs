//! The `PoC` driver: real GPUI window + real Win32 surface container +
//! real native probe editor, wired through the RF388/RF389 contracts.
//!
//! All evidence lines use the `[poc] key=value` form so the run log is
//! the evidence artifact itself.

mod dock;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, GuiHandle, GuiParentToken, ParamInfo, PluginDescriptor,
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
    /// The `GuiParentToken` the redeemer handed `open_gui`.
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

    /// Solid-fill editor color (deep blue) so the attached surface is
    /// visually distinguishable inside the GPUI window.
    const EDITOR_BRUSH: u32 = 0x00_40_20_80;

    /// Window proc delegating to the default — the probe paints
    /// nothing itself (the class background brush fills it).
    ///
    /// # Safety
    /// Called by the OS message loop only, on our own HWND.
    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> LRESULT {
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
            // `hbrBackground` intentionally leaks one brush.
            unsafe {
                let class = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: GetModuleHandleW(core::ptr::null()),
                    hIcon: core::ptr::null_mut(),
                    hCursor: core::ptr::null_mut(),
                    hbrBackground: CreateSolidBrush(EDITOR_BRUSH),
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

    fn open_gui(&mut self, parent: GuiParentToken) -> Result<GuiHandle, HostError> {
        self.shared.parent_token.set(parent.raw());
        self.shared.opens.set(self.shared.opens.get() + 1);
        let _child = probe::create_editor(parent.raw(), &self.shared)?;
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

/// Emits one `[poc] key=value` evidence line.
fn ev(line: &str) {
    eprintln!("[poc] {line}");
}

/// Builds the session ops with the consumer-side redeemer — the
/// composition the architecture baseline reserves for this crate:
/// the adapter's live-lease validation turns an opaque
/// [`ParentSlotLease`] payload into the native [`GuiParentToken`].
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
                .map(GuiParentToken::new)
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
