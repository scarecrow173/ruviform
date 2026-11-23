//! The runner's child-owned floating-GUI mechanism (wire v6 — the
//! Issue #872 design, slice 2 / Issue #883): a lazily spawned UI
//! thread owning the Win32 message pump and the container toplevel
//! each instance's editor attaches into, plus the marshal seam that
//! hands GUI verbs from the command thread to that UI thread.
//!
//! ## Topology
//!
//! Every GUI verb (`OpenGui`/`CloseGui`/`SetGuiSize`/
//! `SetGuiVisibility`/`SetGuiFocus`/`SendGuiKey`) is instance-scoped —
//! the envelope `instance` alone names the surface (≤1 surface per
//! instance, so no `GuiHandle` rides a command payload). The command
//! thread resolves the handle and the surface bookkeeping
//! (`InstanceEntry::gui_handle`), then marshals the verb here: the UI
//! thread owns every window object and every plugin GUI hook call.
//! The reply waits are **unbounded** by design — the bound lives on
//! the parent side (`command_timeout` → kill); a child-side GUI-op
//! deadline would only hide a wedged hook whose editor is already
//! lost (design, Risks and Trade-offs).
//!
//! The container is an owned toplevel (`WS_OVERLAPPED` family),
//! **never** `SetParent`/`WS_CHILD` — the cross-process child
//! embedding Issue #115 rejected. `OpenGui.parent` carries the host
//! HWND only as an owner hint: `payload != 0` applies
//! `SetWindowLongPtrW(GWLP_HWNDPARENT)` best-effort — for an
//! overlapped window that slot *is* the owner handle — coupling
//! Z-order/minimize behavior and nothing else; it does not attach
//! the two windows' input queues (the `AttachThreadInput` coupling
//! #115 bans rides `SetParent`/`WS_CHILD`, not owned toplevels).
//! The plugin's own `open_gui` receives a runner-minted
//! [`ParentLease`] whose payload is the **container** HWND — a
//! plugin that `WS_CHILD`-attaches (the `test_editor` path) then
//! parents inside this process, never across it.
//!
//! ## `unsafe` inventory
//!
//! `unsafe` stays inside this module (`#[expect(unsafe_code)]` on the
//! `mod` declaration in lib.rs): the Win32 calls below, and the one
//! [`SendPluginPtr`] handoff — see its SAFETY comment for the
//! exclusive-borrow argument.
//!
//! **RT contract:** everything here is non-RT only — the UI thread
//! serves the control plane, never the audio leg.

use std::collections::HashMap;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, mpsc};
use std::thread::{self, JoinHandle};

use ruviform_plugin_core::wire::{self, Command as WireCommand, Reply as WireReply};
use ruviform_plugin_core::{ErasedPlugin, GuiSize, HostError, ParentLease};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleHandleExW,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWLP_HWNDPARENT, GetClassInfoW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW,
    PostThreadMessageW, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOZORDER,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, WM_APP, WM_USER, WNDCLASSW,
    WS_CLIPCHILDREN, WS_OVERLAPPED,
};

/// The `Opened.capabilities` bitfield this runner reports — `float`
/// (bit 1) + `close` (bit 7) = `0x82`, in `EditorCapabilities` field
/// declaration order (`dock`/`float`/`resize`/`hide_show`/`reparent`/
/// `ime`/`popup`/`close`/`input`/`focus`). Both bits are
/// runner-mechanism facts — an accepted open proves the child can
/// float and close — so the value is constant: `ErasedPlugin`
/// carries no per-plugin capability query hook (`open_gui` returns a
/// bare `GuiHandle`), and per-plugin `resize`/`hide_show`/`input`/
/// `focus` support stays unclaimed until the verb executes — the L0
/// `editor_session.rs` precedent ("accepted open is the
/// caps-discovery path"). `dock`/`reparent`/`ime`/`popup` are `0` —
/// the docked family and IME are out of the wire-v6 design scope.
const OPENED_CAPABILITIES: u32 = 0x82;

/// The thread-queue message that wakes the UI thread's pump to drain
/// the marshal channel — `WM_APP`-range (app-private, never a window
/// message).
const WM_GUI_WAKE: u32 = WM_APP + 1;

/// The command thread's handle on the lazily spawned UI thread.
///
/// `Some` on the runner means a live UI thread with a message pump;
/// `None` means no GUI verb has ever needed one — the lazy-start
/// invariant that keeps an audio-only child identical to today.
pub(crate) struct Ui {
    /// Marshal channel — every GUI verb/teardown request travels it.
    tx: mpsc::Sender<GuiRequest>,
    /// The UI thread's Win32 id — the `PostThreadMessageW` wake
    /// target, handed back at spawn after its message queue exists.
    tid: u32,
    /// The thread handle `shutdown` joins.
    join: JoinHandle<()>,
}

impl Ui {
    /// Spawns the UI thread and waits for its message queue to exist —
    /// a `PostThreadMessageW` before the queue exists is dropped, so
    /// the thread reports its id only after `PeekMessageW` forced the
    /// queue into being. `Err(Ipc)` on a spawn/startup failure — the
    /// same child-side OS-resource vocabulary as shm attach.
    pub(crate) fn spawn() -> Result<Self, HostError> {
        let (tx, rx) = mpsc::channel::<GuiRequest>();
        let (ready_tx, ready_rx) = mpsc::channel::<u32>();
        let join = thread::Builder::new()
            .name("rvf-l1-gui".into())
            .spawn(move || ui_thread_main(rx, ready_tx))
            .map_err(|_| HostError::Ipc)?;
        let tid = ready_rx.recv().map_err(|_| HostError::Ipc)?;
        Ok(Self { tx, tid, join })
    }

    /// Marshals one GUI verb onto the UI thread and parks until its
    /// reply arrives — unbounded by design (the bound is the parent's
    /// `command_timeout`; a wedged hook stalls this wait and the
    /// child with it, which is the documented wedge convergence).
    /// `Err(Ipc)` when the marshal transport is gone (dead UI thread —
    /// its dropped reply sender ends the wait either way).
    pub(crate) fn verb(
        &self,
        instance: u32,
        plugin: &mut (dyn ErasedPlugin + 'static),
        command: WireCommand,
    ) -> Result<WireReply, HostError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(GuiRequest::Verb {
                instance,
                plugin: SendPluginPtr(plugin),
                command,
                reply: reply_tx,
            })
            .map_err(|_| HostError::Ipc)?;
        wake_ui_thread(self.tid);
        reply_rx.recv().map_err(|_| HostError::Ipc)?
    }

    /// Marshals a best-effort surface teardown for `instance`
    /// (`DestroyInstance`/`normalize_all`): the plugin's `close_gui`
    /// plus the container destroy. Never fails — a dead UI thread
    /// simply skips the marshal, and a wedged one holds this wait
    /// forever, which the wedge contract accepts: the process stays
    /// and the parent's `command_timeout` → kill converges it.
    pub(crate) fn close_surface(&self, instance: u32, plugin: &mut (dyn ErasedPlugin + 'static)) {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .tx
            .send(GuiRequest::CloseSurface {
                instance,
                plugin: SendPluginPtr(plugin),
                reply: reply_tx,
            })
            .is_err()
        {
            return;
        }
        wake_ui_thread(self.tid);
        let _ = reply_rx.recv();
    }

    /// Drains the UI thread — every remaining container is destroyed —
    /// then joins it. Called once from `normalize_all` on every exit
    /// path; a wedged UI thread never returns from this (the wedged-
    /// join specification: the child stays put for the parent to
    /// kill).
    pub(crate) fn shutdown(self) {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .tx
            .send(GuiRequest::Shutdown { reply: reply_tx })
            .is_ok()
        {
            wake_ui_thread(self.tid);
            let _ = reply_rx.recv();
        }
        let _ = self.join.join();
    }
}

/// Posts the wake message to the UI thread's queue. A failed post is
/// harmless: it means the thread is gone, and the queued request's
/// reply sender drops with the dead `Receiver`, ending the caller's
/// wait as a defined `Err(Ipc)` — so the post result is unchecked.
fn wake_ui_thread(tid: u32) {
    // SAFETY: `PostThreadMessageW` accepts any thread id, live or
    // dead — failure is reported through the return code only.
    unsafe {
        PostThreadMessageW(tid, WM_GUI_WAKE, 0, 0);
    }
}

/// One marshal request from the command thread to the UI thread.
enum GuiRequest {
    /// A GUI verb to run against `instance`'s surface — or to open
    /// one (`OpenGui`). Carries the plugin pointer for the hook calls.
    Verb {
        /// The wire envelope `instance` — the surfaces map key.
        instance: u32,
        /// The instance's plugin, hand-borrowed for this request.
        plugin: SendPluginPtr,
        /// The GUI `wire::Command` variant verbatim.
        command: WireCommand,
        /// Oneshot the reply rides back on.
        reply: mpsc::Sender<Result<WireReply, HostError>>,
    },
    /// Best-effort teardown of `instance`'s surface — plugin
    /// `close_gui` plus container destroy — for `DestroyInstance` and
    /// `normalize_all`.
    CloseSurface {
        /// The wire envelope `instance` — the surfaces map key.
        instance: u32,
        /// The instance's plugin, hand-borrowed for this request.
        plugin: SendPluginPtr,
        /// Oneshot signaling the teardown finished.
        reply: mpsc::Sender<()>,
    },
    /// Process teardown — destroy every remaining container and end
    /// the pump so `shutdown`'s join returns.
    Shutdown {
        /// Oneshot signaling the drain finished.
        reply: mpsc::Sender<()>,
    },
}

/// A `*mut dyn ErasedPlugin` handoff between the command thread and
/// the UI thread. `Box<dyn ErasedPlugin>` is deliberately not `Send`
/// (adapters hold thread-affine state — `create_sendable` is opt-in
/// and VST3-class plugins decline it), so the marshal needs this
/// scoped wrapper; `thread::scope` cannot cover a persistent thread.
#[derive(Copy, Clone)]
struct SendPluginPtr(*mut dyn ErasedPlugin);

// SAFETY: the command thread is parked on the request's reply
// oneshot for the entire lifetime of the handoff — only inside that
// parked window does the UI thread dereference the pointer, and it
// holds the exclusive borrow for exactly the request's duration,
// discarding the pointer when the reply sends and never touching it
// again. While a marshal is in flight the single-threaded command
// loop services no other command, so no other access to the same
// `InstanceEntry` can exist. If a hook wedges, the process dies with
// the parent's `command_timeout` → kill convergence, so the pointer
// can never dangle.
unsafe impl Send for SendPluginPtr {}

/// The UI thread body: the instance → container bookkeeping and the
/// pump that serves both Win32 messages and the marshal channel.
/// `rx`/`ready` are owned — the `move`d spawn closure transfers
/// them; channel ops borrow `&self` so nothing here "consumes" them
/// by value-shape, hence the scoped permit.
#[expect(clippy::needless_pass_by_value)]
fn ui_thread_main(rx: mpsc::Receiver<GuiRequest>, ready: mpsc::Sender<u32>) {
    // Force the thread's message queue into existence before
    // reporting readiness — `PostThreadMessageW` against a thread
    // with no queue is silently dropped.
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out-pointer; the NULL hwnd and the
    // NOREMOVE flag make this a pure queue-creation probe.
    unsafe {
        PeekMessageW(&raw mut msg, ptr::null_mut(), WM_USER, WM_USER, PM_NOREMOVE);
    }
    // SAFETY: called on the thread whose id is being read.
    let tid = unsafe { GetCurrentThreadId() };
    let _ = ready.send(tid);

    let mut surfaces: HashMap<u32, HWND> = HashMap::new();
    loop {
        // SAFETY: `msg` is a valid out-pointer; NULL hwnd receives
        // this thread's window and thread-queue messages.
        let got = unsafe { GetMessageW(&raw mut msg, ptr::null_mut(), 0, 0) };
        if got <= 0 {
            // WM_QUIT (0) or an error (-1) — no normal path posts
            // either; leave the loop for the drain below.
            break;
        }
        // SAFETY: `msg` holds the message `GetMessageW` just
        // delivered; both calls are the standard pump shape.
        unsafe {
            TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
        // Any message — the wake post included — is a drain point:
        // every queued marshal request is serviced, oldest first.
        while let Ok(request) = rx.try_recv() {
            match request {
                GuiRequest::Verb {
                    instance,
                    plugin,
                    command,
                    reply,
                } => {
                    let _ = reply.send(run_verb(&mut surfaces, instance, plugin, command));
                }
                GuiRequest::CloseSurface {
                    instance,
                    plugin,
                    reply,
                } => {
                    close_surface(&mut surfaces, instance, plugin);
                    let _ = reply.send(());
                }
                GuiRequest::Shutdown { reply } => {
                    destroy_all(&mut surfaces);
                    let _ = reply.send(());
                    return;
                }
            }
        }
    }
    // The pump exited without a `Shutdown` request — best-effort
    // container cleanup; the defined path always exits through
    // `Shutdown` above.
    destroy_all(&mut surfaces);
}

/// Runs one GUI verb on the UI thread and produces its wire reply.
/// `plugin` is the exclusive borrow the marshal handoff supplies —
/// valid only for this call. `command` is by value (the verb is
/// consumed semantics) though every payload field is `Copy`, hence
/// the scoped permit.
#[expect(clippy::needless_pass_by_value)]
fn run_verb(
    surfaces: &mut HashMap<u32, HWND>,
    instance: u32,
    plugin: SendPluginPtr,
    command: WireCommand,
) -> Result<WireReply, HostError> {
    // SAFETY: the command thread is parked on this request's reply
    // oneshot — the exclusive-borrow window the `SendPluginPtr`
    // SAFETY contract licenses. The reference lives only inside this
    // call and is never stored.
    let plugin = unsafe { &mut *plugin.0 };
    match command {
        WireCommand::OpenGui {
            mode: wire::OpenGuiMode::Floating,
            parent,
            size,
        } => open_floating(surfaces, instance, plugin, parent, size),
        WireCommand::CloseGui => {
            // The command thread only marshals a close with a live
            // surface; a missing map entry is the same defined
            // no-surface vocabulary.
            if !surfaces.contains_key(&instance) {
                return Err(HostError::InvalidValue);
            }
            crate::guarded(plugin, ErasedPlugin::close_gui).and_then(|inner| inner)?;
            if let Some(hwnd) = surfaces.remove(&instance) {
                destroy_container(hwnd);
            }
            Ok(WireReply::Ok)
        }
        WireCommand::SetGuiSize { size } => {
            if !surfaces.contains_key(&instance) {
                return Err(HostError::InvalidValue);
            }
            crate::guarded(plugin, |plugin| plugin.set_gui_size(size)).and_then(|inner| inner)?;
            // The container follows the negotiated size — a plugin
            // whose constraint hook adjusted the rect carries the
            // final geometry on its own view, so the runner sizes
            // the container to the requested logical size (the same
            // unit≈pixel deviation the attach path documents).
            if let Some(&hwnd) = surfaces.get(&instance) {
                resize_container(hwnd, size);
            }
            Ok(WireReply::Ok)
        }
        WireCommand::SetGuiVisibility { visible } => {
            // Container show/hide only — the plugin's
            // `set_gui_visibility` hook is deliberately not called:
            // `Hidden`/`Shown` is a container state, not a view
            // detach (D-verbs).
            let Some(&hwnd) = surfaces.get(&instance) else {
                return Err(HostError::InvalidValue);
            };
            set_container_visible(hwnd, visible != 0);
            Ok(WireReply::Ok)
        }
        WireCommand::SetGuiFocus { focused } => {
            if !surfaces.contains_key(&instance) {
                return Err(HostError::InvalidValue);
            }
            crate::guarded(plugin, |plugin| plugin.set_gui_focus(focused != 0))
                .and_then(|inner| inner)?;
            Ok(WireReply::Ok)
        }
        WireCommand::SendGuiKey { event } => {
            if !surfaces.contains_key(&instance) {
                return Err(HostError::InvalidValue);
            }
            crate::guarded(plugin, |plugin| plugin.send_gui_key(event)).and_then(|inner| inner)?;
            Ok(WireReply::Ok)
        }
        // Unreachable — the command thread only marshals the GUI
        // family — kept so `run_verb` is total.
        _ => Err(HostError::Unsupported),
    }
}

/// `OpenGui` with `mode = Floating`: the child-owned container
/// toplevel, the `SetOwner` owner hint, then `plugin.open_gui` under
/// a runner-minted [`ParentLease`] carrying the **container** HWND —
/// the reading that keeps a `WS_CHILD`-attaching plugin (e.g.
/// `test_editor`) inside this process (#115's banned cross-process
/// `SetParent`/`WS_CHILD` never happens). The wire `parent` is
/// consumed only as the owner hint and never reaches the plugin.
fn open_floating(
    surfaces: &mut HashMap<u32, HWND>,
    instance: u32,
    plugin: &mut dyn ErasedPlugin,
    parent: ParentLease,
    size: GuiSize,
) -> Result<WireReply, HostError> {
    if let Some(&hwnd) = surfaces.get(&instance) {
        // A second `OpenGui` on a live surface gets the plugin's own
        // decline — the ≤1-surface rule lives in the hook contract
        // (the VST3 `view.is_some()` → `PluginFailed` precedent),
        // never in a runner-side gate. The hook is still called, with
        // the live container's lease.
        return crate::guarded(plugin, |plugin| {
            plugin.open_gui(ParentLease::new(parent.generation(), hwnd as u64))
        })
        .and_then(|inner| inner)
        .map(|gui| WireReply::Opened {
            gui,
            capabilities: OPENED_CAPABILITIES,
        });
    }
    let Some(hwnd) = create_container(size) else {
        // Child-side OS resource failure — the `Ipc` vocabulary the
        // shm attach precedent established. The open fails defined
        // and the child/UI thread stay alive.
        return Err(HostError::Ipc);
    };
    if parent.payload() != 0 {
        // Best-effort owner hint — a failed association still leaves
        // a valid open (design Risks: strict owner validation is a
        // Deferred concern).
        set_owner(hwnd, parent.payload());
    }
    match crate::guarded(plugin, |plugin| {
        plugin.open_gui(ParentLease::new(parent.generation(), hwnd as u64))
    })
    .and_then(|inner| inner)
    {
        Ok(gui) => {
            surfaces.insert(instance, hwnd);
            Ok(WireReply::Opened {
                gui,
                capabilities: OPENED_CAPABILITIES,
            })
        }
        Err(error) => {
            // The hook declined — fold the container back so no
            // bookkeeping or window is left behind.
            destroy_container(hwnd);
            Err(error)
        }
    }
}

/// `CloseSurface`/`Shutdown` workhorse — best-effort: the plugin's
/// `close_gui` runs (its `Err` is advisory here, the surface folds
/// regardless), the container is destroyed, the map entry clears.
fn close_surface(surfaces: &mut HashMap<u32, HWND>, instance: u32, plugin: SendPluginPtr) {
    if !surfaces.contains_key(&instance) {
        return;
    }
    // SAFETY: same parked-command-thread exclusive borrow as
    // `run_verb` — scoped to this call only.
    let plugin = unsafe { &mut *plugin.0 };
    let _ = crate::guarded(plugin, ErasedPlugin::close_gui);
    if let Some(hwnd) = surfaces.remove(&instance) {
        destroy_container(hwnd);
    }
}

/// Destroys every live container — `Shutdown`/abnormal pump exit.
/// Plugin hooks are never called here: they only ever run inside a
/// request that hand-carries the plugin pointer.
fn destroy_all(surfaces: &mut HashMap<u32, HWND>) {
    for (_instance, hwnd) in surfaces.drain() {
        destroy_container(hwnd);
    }
}

// --- container toplevel -------------------------------------------------

/// The wide container class name — `"RuviformL1GuiContainer"`.
fn class_name() -> Vec<u16> {
    "RuviformL1GuiContainer"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

/// This module's `HINSTANCE`, captured at registration — the
/// `hInstance` argument `CreateWindowExW` reuses. `UNCHANGED_REFCOUNT`
/// keeps the lookup free of refcount side effects; `FROM_ADDRESS`
/// anchors the lookup inside this binary rather than
/// `GetModuleHandleW(NULL)` (which would report a host EXE's module
/// if this code were ever statically linked into one — it is a
/// `.exe` runner, but the cheap precision costs nothing).
///
/// # Safety
/// `class_name` is a function in this module, so its address is a
/// valid `FROM_ADDRESS` anchor.
fn module_instance() -> HINSTANCE {
    let mut module: HINSTANCE = ptr::null_mut();
    // SAFETY: `class_name`'s code address lies inside this module;
    // `module` is a valid out-pointer.
    unsafe {
        let found = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            class_name as *const u16,
            &raw mut module,
        );
        if found == 0 { ptr::null_mut() } else { module }
    }
}

/// Set once the container class registration succeeded (or was found
/// already registered — a second copy of this module in one process
/// cannot exist, but `RegisterClassW` failure is still checked
/// through `GetClassInfoW` like the `test_editor` precedent).
static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Registers the container class once per process. The class's
/// `lpfnWndProc` is `DefWindowProcW` — the container is a
/// message-receiving toplevel that needs no custom behavior, and a
/// `DefWindowProcW` class can never dangle a proc pointer the way a
/// module-local `wnd_proc` could if this code were ever moved into
/// an unloadable module (the `test_editor` rationale, kept).
fn ensure_container_class() -> bool {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let name = class_name();
        // SAFETY: `name`/`class` outlive the calls (Windows copies
        // neither `lpszClassName` nor the struct — the name is kept
        // alive by `mem::forget` below); `hInstance` is this module's
        // own. `hbrBackground` stays NULL — the container is always
        // hidden or covered by the plugin child, so it needs no
        // brush.
        unsafe {
            let hinstance = module_instance();
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(DefWindowProcW),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: name.as_ptr(),
            };
            if RegisterClassW(&raw const class) != 0 {
                CLASS_REGISTERED.store(true, Ordering::Release);
            } else {
                let mut existing: WNDCLASSW = std::mem::zeroed();
                let exists = GetClassInfoW(hinstance, name.as_ptr(), &raw mut existing) != 0;
                CLASS_REGISTERED.store(exists, Ordering::Release);
            }
        }
        std::mem::forget(name);
    });
    CLASS_REGISTERED.load(Ordering::Acquire)
}

/// Creates one invisible container toplevel for `instance`'s editor.
/// `WS_OVERLAPPED | WS_CLIPCHILDREN`, born hidden — `SetGuiVisibility`
/// owns show/hide; `CLIPCHILDREN` keeps the (no-op) container paint
/// out of the plugin child's area. `size` `{0,0}` defers to a
/// default extent (the plugin-preferred-size vocabulary — the
/// container itself is resizable later by `SetGuiSize`).
/// `None` on any Win32 failure — the caller maps it to
/// `Err(HostError::Ipc)`.
fn create_container(size: GuiSize) -> Option<HWND> {
    if !ensure_container_class() {
        return None;
    }
    let name = class_name();
    let (width, height) = container_dimensions(size);
    // SAFETY: the class is registered, `name` outlives the
    // synchronous call (the system does not retain the pointer), and
    // the returned HWND is owned by the caller until
    // `destroy_container`. An overlapped toplevel with no parent,
    // no menu.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            name.as_ptr(),
            ptr::null(),
            WS_OVERLAPPED | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            0,
            width,
            height,
            ptr::null_mut(),
            ptr::null_mut(),
            module_instance(),
            ptr::null(),
        )
    };
    if hwnd.is_null() { None } else { Some(hwnd) }
}

/// The window dimensions for `size` — `{0,0}` (the
/// plugin-preferred-size marker) maps to `CW_USEDEFAULT`; a nonzero
/// logical size maps 1:1 onto device pixels, the same unit≈pixel
/// deviation the `GuiSize` → `ViewRect` mapping already documents.
fn container_dimensions(size: GuiSize) -> (i32, i32) {
    if size.width > 0.0 && size.height > 0.0 {
        (gui_len(size.width), gui_len(size.height))
    } else {
        (CW_USEDEFAULT, 0)
    }
}

/// Logical-unit → device-pixel conversion with a sane floor — a
/// non-finite or out-of-range size clamps rather than panics (the
/// wire `GuiSize` carries `f64`s whose decode only bounds finiteness
/// through the codec's own checks).
fn gui_len(logical: f64) -> i32 {
    #[expect(clippy::cast_possible_truncation)]
    {
        if logical.is_finite() {
            logical.round().clamp(1.0, f64::from(i32::MAX)) as i32
        } else {
            1
        }
    }
}

/// `SetWindowLongPtrW(GWLP_HWNDPARENT)` — the owner association the
/// floating `parent` hint carries. `GWLP_HWNDPARENT` couples Z-order and
/// minimize/restore behavior **only**: it never attaches the two
/// windows' input queues — the `AttachThreadInput` coupling Issue
/// #115 bans rides `SetParent`/`WS_CHILD`, not owned toplevels
/// (design, Verification Notes — reconfirmed against the Win32
/// docs). Best-effort: the return (the previous owner) is unchecked
/// and a failed association leaves the open valid.
fn set_owner(hwnd: HWND, owner_payload: u64) {
    // SAFETY: `hwnd` is a live container this module created; the
    // owner value is an opaque handle token — Windows validates it.
    unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWLP_HWNDPARENT,
            isize::try_from(owner_payload).unwrap_or(isize::MAX),
        );
    }
}

/// `ShowWindow(SW_SHOW/SW_HIDE)` — the container visibility the
/// `SetGuiVisibility` verb owns (the plugin view itself survives
/// either way).
fn set_container_visible(hwnd: HWND, visible: bool) {
    // SAFETY: `hwnd` is a live container this module created.
    unsafe {
        ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
    }
}

/// `SetWindowPos` size-only — keeps position and Z-order.
fn resize_container(hwnd: HWND, size: GuiSize) {
    // SAFETY: `hwnd` is a live container this module created;
    // `SetWindowPos` on an already-dead handle is a defined no-op
    // error.
    unsafe {
        SetWindowPos(
            hwnd,
            ptr::null_mut(),
            0,
            0,
            gui_len(size.width),
            gui_len(size.height),
            SWP_NOMOVE | SWP_NOZORDER,
        );
    }
}

/// `DestroyWindow` — must run on the UI thread that owns the HWND
/// (which is exactly where every caller of this sits).
fn destroy_container(hwnd: HWND) {
    // SAFETY: `hwnd` is a container this module created on this
    // thread; calling on the owning thread is the `DestroyWindow`
    // contract.
    unsafe {
        DestroyWindow(hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GWLP_HWNDPARENT` round-trip — the headless-checkable bound of the
    /// `SetOwner` non-coupling contract: the association write reads
    /// back verbatim through `GetWindowLongPtrW`, pinning that what
    /// was applied is the owner association and only that (input-
    /// queue coupling would need `SetParent`/`AttachThreadInput`,
    /// which this code never calls). A session-0 window station that
    /// cannot create the containers is an honest environment skip,
    /// not a failure — interactive re-verification stays with the
    /// lane work (#886).
    #[test]
    fn gwlp_owner_association_reads_back() {
        let default = GuiSize {
            width: 0.0,
            height: 0.0,
        };
        let Some(owner) = create_container(default) else {
            eprintln!(
                "gui tests: container creation unavailable in this session — GWLP_HWNDPARENT read-back not run"
            );
            return;
        };
        let Some(hwnd) = create_container(default) else {
            destroy_container(owner);
            eprintln!(
                "gui tests: second container unavailable — GWLP_HWNDPARENT read-back not run"
            );
            return;
        };
        let owner_isize = isize::try_from(owner as u64).expect("HWND fits isize");
        set_owner(hwnd, owner as u64);
        // SAFETY: `hwnd` is a live container created above.
        let read_back = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT)
        };
        assert_eq!(
            read_back, owner_isize,
            "GWLP_HWNDPARENT must read back the association verbatim"
        );
        destroy_container(hwnd);
        destroy_container(owner);
    }
}
