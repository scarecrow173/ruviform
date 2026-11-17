//! The `test_editor` CLAP plugin: a parameterless pass-through
//! processor plus the plugin side of `clap.ext.gui` — the CLAP half
//! of Issue #630 AC-4 ("Native Editor が実 child HWND を生成できる"),
//! mirroring the VST3 `test_editor` fixture
//! (`ruviform-plugin-test-vst3`'s `editor.rs`/`editor_view.rs`).
//!
//! ## DSP/param law
//!
//! Identical in law to `test_fault`'s unarmed mode: stereo in/out,
//! `out == in` bitwise, **no parameters** — the `params` extension is
//! not declared at all, so foreign `ParamValue` events are dropped at
//! the `block::translate_events` boundary and can never affect the
//! output. The processor is a unit struct: there is no armed state to
//! persist.
//!
//! ## GUI contract (what the production host's `open_gui` requires)
//!
//! `ClapPlugin::open_gui` (`ruviform-plugin-clap` `host.rs`,
//! unchanged substrate) calls, in order: `get_extension::<PluginGui>()`
//! → `is_api_supported(WIN32, is_floating=false)` → `create` →
//! `set_parent(Window::from_win32_hwnd(parent.payload()))` → `show`;
//! failures roll back through `destroy` and surface `HostError::
//! PluginFailed`. `close_gui` runs `hide` (best-effort) + `destroy`.
//! Non-Windows builds answer `HostError::Unsupported` before any
//! extension call (`EDITOR_GUI_API = None`).
//!
//! This fixture therefore:
//!
//! - declares `PluginGui` via `declare_extensions` and implements
//!   [`PluginGuiImpl`] on the main-thread object;
//! - answers `is_api_supported`/`get_preferred_api` true only for
//!   `win32` + embedded on Windows builds — floating windows,
//!   other APIs, and non-Windows builds are honest `false`/`None`;
//! - `set_parent` validates the supplied [`Window`] is a Win32
//!   window, requires `create` first, rejects a second embed, then
//!   creates a real `WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS` child
//!   HWND under the parent — an opaque lease payload such as
//!   `ParentLease::new(0, 0)` (payload `0`) fails the `IsWindow`
//!   gate and is rejected, never treated as a window;
//! - `show`/`hide` toggle the child's visibility; `destroy` is
//!   idempotent and frees the child HWND;
//! - size verbs are honest: `get_size` reports the current size
//!   (640x480 until `set_size`), `can_resize` accepts and `set_size`
//!   records the requested size and resizes the attached child
//!   HWND — the VST3 `test_editor` `canResize`/`onSize` counterpart;
//!   `adjust_size`/`get_resize_hints` keep the trait defaults (no
//!   constraints to declare), `set_size` before `create` is a
//!   defined rejection, `set_transient` rejects (floating
//!   unsupported), `set_scale` acknowledges a finite positive
//!   factor (CLAP permits ignoring it — the Win32 child uses
//!   physical pixels), `suggest_title` is a documented no-op.
//!
//! ## Windows child window
//!
//! Win32 calls are confined to the `#[cfg(target_os = "windows")]`
//! `win32` module behind `#[expect(unsafe_code)]` + SAFETY comments;
//! off Windows the same module is a shim returning `None`/no-op so
//! every verb converges to its defined rejection and the cdylib
//! still builds. The window class registers once per module with
//! `lpfnWndProc = DefWindowProcW` deliberately — a cdylib-registered
//! class **outlives the module** (Windows does not unregister DLL
//! classes on unload), so a module-local `wnd_proc` would dangle
//! after `FreeLibrary` and crash the host on the next message
//! dispatch, including on reload where `GetClassInfoW` would adopt
//! the stale class. `DefWindowProcW` lives in `user32`, which never
//! unloads. Registration follows the VST3 fixture's precedent
//! (`GetClassInfoW` confirmation, `GetModuleHandleExW` own-module
//! `HINSTANCE`) with a distinct class name
//! (`"RuviformClapTestEditor"`) and background color so the two
//! fixtures' surfaces stay distinguishable in one process. There is
//! **no message pump or thread of our own** — the host calls the
//! gui verbs on the thread that owns the parent HWND.
//!
//! GUI state lives on [`TestEditorMainThread`] behind a `RefCell`
//! (`PluginGuiImpl` methods take `&self`); re-entrant borrows are a
//! defined `PluginError` rejection, the same policy the VST3 view
//! applies. GUI resources are torn down by `destroy` — the host
//! bridge runs `hide` + `destroy` before `plugin.destroy`, so the
//! child HWND never outlives its instance under the contract.

use std::cell::RefCell;
use std::marker::PhantomData;

use clack_common::plugin::features;
use clack_common::utils::ClapId;
use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_extensions::gui::{
    GuiApiType, GuiConfiguration, GuiSize, PluginGui, PluginGuiImpl, Window,
};
use clack_plugin::extensions::PluginExtensions;
use clack_plugin::host::{HostAudioProcessorHandle, HostMainThreadHandle, HostSharedHandle};
use clack_plugin::plugin::{
    Plugin, PluginAudioProcessor, PluginDescriptor, PluginError, PluginMainThread, PluginShared,
};
use clack_plugin::process::{
    Audio, Events, PluginAudioConfiguration, Process, ProcessStatus as ClapProcessStatus,
};

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, LifecycleEvent, PluginProcessor, PrepareError,
    ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

use crate::block::{self, AtomicLifecycle, BlockScratch, BlockViews};
use crate::fault::pass_through;

// ---------------------------------------------------------------------------
// GUI constants + state
// ---------------------------------------------------------------------------

/// The editor's initial size — `get_size` reports the current size
/// (this default until `set_size` replaces it); resize is accepted
/// (`can_resize` → `true`, `set_size` resizes the child HWND).
const DEFAULT_GUI_SIZE: GuiSize = GuiSize {
    width: 640,
    height: 480,
};

/// The `win32` + embedded configuration this fixture supports — the
/// only [`GuiConfiguration`] `is_api_supported` ever answers `true`
/// for (and only on Windows builds).
const EMBEDDED_WIN32: GuiConfiguration<'static> = GuiConfiguration {
    api_type: GuiApiType::WIN32,
    is_floating: false,
};

/// Interior-mutable GUI state — every [`PluginGuiImpl`] method takes
/// `&self`, so the fields live behind `RefCell` (the VST3
/// `editor_view.rs` `ViewState` precedent).
#[derive(Debug)]
struct GuiState {
    /// `create` ran and `destroy` has not — state verbs gate on this.
    created: bool,
    /// The embedded child HWND as a `usize` (`None` until
    /// `set_parent`). Only ever `Some` on Windows — the non-Windows
    /// `win32` shim never produces one.
    hwnd: Option<usize>,
    /// The current editor size — `get_size` reports it, `set_size`
    /// replaces it (and resizes the child HWND when attached).
    size: GuiSize,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            created: false,
            hwnd: None,
            size: DEFAULT_GUI_SIZE,
        }
    }
}

// ---------------------------------------------------------------------------
// Processor (neutral contract — no clack types)
// ---------------------------------------------------------------------------

/// The `test_editor` processor — pure pass-through, parameterless.
/// A unit struct: there is no armed/gain state to hold.
///
/// **RT contract:** RT-suitable — `Copy` marker.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct TestEditorProcessor;

impl PluginProcessor for TestEditorProcessor {
    /// Activation hook — nothing to configure for a stateless
    /// pass-through.
    ///
    /// **RT contract:** non-RT only — may allocate; this
    /// implementation does not need to.
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    /// One block of bitwise pass-through. `events` is ignored — the
    /// fixture declares no parameter surface, so every event kind is
    /// inert here (the `|_| None` translator upstream already drops
    /// them all; this is the second, independent guarantee).
    ///
    /// **RT contract:** RT-suitable — bounded copy, no allocation.
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let frames = inputs
            .first()
            .map_or(0, BufferView::frames)
            .min(outputs.first().map_or(0, BufferView::frames));
        pass_through(inputs, outputs, frames);
        if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        }
    }
}

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine. No parameter mirror — the fixture
/// has no parameters.
pub(crate) struct TestEditorShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
}

impl PluginShared<'_> for TestEditorShared {}

impl TestEditorShared {
    /// The `plugin.init` hook's fallible core: Created → Initialize
    /// is unconditional for this stateless init.
    ///
    /// **RT contract:** non-RT only — runs at instance creation.
    ///
    /// # Errors
    ///
    /// `PluginError::Message` — the lifecycle machine rejected
    /// `Initialize` (a re-init host protocol violation).
    fn initialize() -> Result<Self, PluginError> {
        let shared = Self {
            lifecycle: AtomicLifecycle::new(),
        };
        shared
            .lifecycle
            .transition(LifecycleEvent::Initialize)
            .map_err(|_| PluginError::Message("init outside Created state"))?;
        Ok(shared)
    }

    /// The `plugin.destroy` transition — invoked from [`Drop`].
    fn destroy(&self) {
        let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
    }
}

impl Drop for TestEditorShared {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The plugin's main-thread object (`[main-thread]`): audio-port
/// metadata and the `clap.ext.gui` state live here.
///
/// Unlike the sibling fixtures this object reads no `Shared` state —
/// the gui verbs are self-contained in [`GuiState`] — so the shared
/// borrow is carried as a lifetime marker only.
pub(crate) struct TestEditorMainThread<'a> {
    gui: RefCell<GuiState>,
    _shared: PhantomData<&'a TestEditorShared>,
}

impl<'a> PluginMainThread<'a, TestEditorShared> for TestEditorMainThread<'a> {}

/// The audio-thread object (`[audio-thread]`): owns the neutral
/// [`TestEditorProcessor`], its context and all per-block scratch.
pub(crate) struct TestEditorAudioProcessor<'a> {
    shared: &'a TestEditorShared,
    processor: TestEditorProcessor,
    context: ProcessContext,
    config: ProcessConfig,
    scratch: BlockScratch,
}

// ---------------------------------------------------------------------------
// Plugin type + factory
// ---------------------------------------------------------------------------

/// The `test_editor` plugin type — a unit struct tying the associated
/// types together, instantiated through the suite's custom
/// [`clack_plugin::entry::Entry`] factory.
///
/// **RT contract:** RT-suitable — `Copy` marker type.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TestEditorClapPlugin;

impl Plugin for TestEditorClapPlugin {
    type AudioProcessor<'a> = TestEditorAudioProcessor<'a>;
    type Shared<'a> = TestEditorShared;
    type MainThread<'a> = TestEditorMainThread<'a>;

    fn declare_extensions(
        builder: &mut PluginExtensions<Self>,
        _shared: Option<&TestEditorShared>,
    ) {
        // `PluginGui` is the only non-audio extension — no `params`,
        // no `state`: the fixture's parameter surface is honestly
        // absent rather than declared-empty.
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginGui>();
    }
}

impl clack_plugin::entry::DefaultPluginFactory for TestEditorClapPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.ruviform-test.clap-test-editor", "test_editor")
            .with_vendor("Ruviform Test")
            .with_version("0.0.0")
            .with_description(
                "Ruviform test-suite CLAP fixture — clap.ext.gui editor surface for host GUI verification",
            )
            .with_features([features::AUDIO_EFFECT, features::STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<TestEditorShared, PluginError> {
        // `plugin.init` — the host handle is intentionally not stored:
        // this fixture declares no host-facing extensions that need
        // it after construction.
        TestEditorShared::initialize()
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        _shared: &'a TestEditorShared,
    ) -> Result<TestEditorMainThread<'a>, PluginError> {
        Ok(TestEditorMainThread {
            gui: RefCell::new(GuiState::default()),
            _shared: PhantomData,
        })
    }
}

// ---------------------------------------------------------------------------
// Audio ports extension (main thread)
// ---------------------------------------------------------------------------

impl PluginAudioPortsImpl for TestEditorMainThread<'_> {
    fn count(&self, _is_input: bool) -> u32 {
        1
    }

    fn get(&self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index != 0 {
            return;
        }
        writer.set(&AudioPortInfo {
            // Both directions share id 0 — legal per CLAP (ids must be
            // unique *within* a direction) and required for
            // `in_place_pair` pairing.
            id: ClapId::new(0),
            name: if is_input {
                b"Audio In" as &[u8]
            } else {
                b"Audio Out" as &[u8]
            },
            channel_count: 2,
            flags: AudioPortFlags::IS_MAIN,
            port_type: Some(AudioPortType::STEREO),
            in_place_pair: Some(ClapId::new(0)),
        });
    }
}

// ---------------------------------------------------------------------------
// GUI extension (`clap.ext.gui`, plugin side)
// ---------------------------------------------------------------------------

impl PluginGuiImpl for TestEditorMainThread<'_> {
    /// Only `win32` + embedded on Windows builds — floating windows,
    /// other windowing APIs, and non-Windows builds are honest
    /// `false`. The host bridge's `open_gui` requires exactly this
    /// gate before it calls `create`.
    fn is_api_supported(&self, configuration: GuiConfiguration) -> bool {
        configuration.api_type == GuiApiType::WIN32 && !configuration.is_floating && cfg!(windows)
    }

    /// The preferred configuration mirrors `is_api_supported` —
    /// `win32` embedded on Windows, `None` elsewhere (a non-Windows
    /// host must not be hinted toward an API this build cannot
    /// create).
    fn get_preferred_api(&self) -> Option<GuiConfiguration<'_>> {
        cfg!(windows).then_some(EMBEDDED_WIN32)
    }

    /// Marks the GUI created — resource allocation for this fixture
    /// is deferred to `set_parent` (the child HWND is the only real
    /// resource), so `create` validates the negotiated configuration
    /// and records the lifecycle edge. Double `create` and
    /// unsupported configurations are defined rejections.
    fn create(&self, configuration: GuiConfiguration) -> Result<(), PluginError> {
        if !self.is_api_supported(configuration) {
            return Err(PluginError::Message("unsupported gui configuration"));
        }
        let mut state = self
            .gui
            .try_borrow_mut()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        if state.created {
            return Err(PluginError::Message("gui already created"));
        }
        state.created = true;
        Ok(())
    }

    /// Frees the GUI: destroys the child HWND (if any), resets the
    /// created edge and the size state — a recreated GUI starts at
    /// the default size again, the same fresh-view semantics the
    /// VST3 fixture's per-`createView` state gives — and is
    /// idempotent, so a host `destroy` after a failed
    /// `set_parent`/`show` rollback is safe.
    fn destroy(&self) {
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return;
        };
        if let Some(hwnd) = state.hwnd.take() {
            win32::destroy(hwnd);
        }
        state.created = false;
        state.size = DEFAULT_GUI_SIZE;
    }

    /// A finite positive scale is acknowledged and intentionally not
    /// applied — CLAP permits ignoring `set_scale` when the plugin
    /// does not do its own DPI math; the Win32 child uses physical
    /// pixels inside the host-owned parent surface. Non-finite or
    /// non-positive factors are defined rejections.
    fn set_scale(&self, scale: f64) -> Result<(), PluginError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(PluginError::Message("invalid gui scale"));
        }
        Ok(())
    }

    /// The current editor size — 640x480 until `set_size` replaces it.
    fn get_size(&self) -> Option<GuiSize> {
        self.gui.try_borrow().ok().map(|state| state.size)
    }

    // `adjust_size`/`get_resize_hints` keep the trait defaults
    // (`None`/`None`) — the fixture declares no resize constraints.

    /// The fixture's editor genuinely resizes — `set_size` moves the
    /// attached child HWND to the requested size, so `can_resize`
    /// answers `true` (the VST3 `canResize` counterpart).
    fn can_resize(&self) -> bool {
        true
    }

    /// `set_size` records the requested size — `get_size` echoes it
    /// afterwards — and resizes the attached child HWND when one
    /// exists. A `set_size` before `create` is a host protocol
    /// violation and a defined rejection.
    fn set_size(&self, size: GuiSize) -> Result<(), PluginError> {
        let mut state = self
            .gui
            .try_borrow_mut()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        if !state.created {
            return Err(PluginError::Message("set_size before create"));
        }
        state.size = size;
        if let Some(hwnd) = state.hwnd {
            win32::resize(
                hwnd,
                i32::try_from(size.width).unwrap_or(i32::MAX),
                i32::try_from(size.height).unwrap_or(i32::MAX),
            );
        }
        Ok(())
    }

    /// Embeds the GUI: the supplied [`Window`] must be a Win32 window
    /// whose handle passes the `IsWindow` gate (an opaque
    /// `ParentLease` payload such as `0` is *not* a window and is
    /// rejected here), `create` must have run, and no embed may exist
    /// yet. On success a real `WS_CHILD` HWND is created under the
    /// parent; on any failure nothing is created and the host's
    /// `destroy` rollback stays a no-op.
    fn set_parent(&self, window: Window) -> Result<(), PluginError> {
        let parent = window
            .as_win32_hwnd()
            .ok_or(PluginError::Message("set_parent: not a win32 window"))?;
        let mut state = self
            .gui
            .try_borrow_mut()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        if !state.created {
            return Err(PluginError::Message("set_parent before create"));
        }
        if state.hwnd.is_some() {
            return Err(PluginError::Message("gui already embedded"));
        }
        let child = win32::attach(parent).ok_or(PluginError::Message(
            "set_parent: child window creation failed",
        ))?;
        state.hwnd = Some(child);
        Ok(())
    }

    /// Floating windows are out of scope — always a defined
    /// rejection, on every platform.
    fn set_transient(&self, _window: Window) -> Result<(), PluginError> {
        Err(PluginError::Message("floating gui unsupported"))
    }

    // `suggest_title` keeps the trait's no-op default — it only
    // applies to floating windows, which this fixture rejects.

    /// Shows the embedded child (`SW_SHOW`). Requires an attached
    /// parent — `show` before `set_parent` (or before `create`) is a
    /// host protocol violation and a defined rejection.
    fn show(&self) -> Result<(), PluginError> {
        let state = self
            .gui
            .try_borrow()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        let Some(hwnd) = state.hwnd else {
            return Err(PluginError::Message("show before set_parent"));
        };
        win32::set_visible(hwnd, true);
        Ok(())
    }

    /// Hides the embedded child (`SW_HIDE`) without freeing GUI
    /// resources — `destroy` owns teardown. Same attach requirement
    /// as `show`.
    fn hide(&self) -> Result<(), PluginError> {
        let state = self
            .gui
            .try_borrow()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        let Some(hwnd) = state.hwnd else {
            return Err(PluginError::Message("hide before set_parent"));
        };
        win32::set_visible(hwnd, false);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Windows child-window management — the only Win32 surface in this
// crate. Mirrors the VST3 `editor_view.rs` `win32` module: once-per-
// module class registration with `DefWindowProcW` (the class
// outlives this cdylib — a module-local `wnd_proc` would dangle after
// `FreeLibrary`), `GetClassInfoW` confirmation, own-module
// `HINSTANCE` via `GetModuleHandleExW`, no message pump or thread.
// ---------------------------------------------------------------------------

/// Windows implementation — real child HWND create/destroy/show.
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
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassInfoW,
        GetClientRect, IsWindow, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOZORDER,
        SetWindowPos, ShowWindow, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
    };

    /// Solid-fill color — blue, distinct from the VST3 fixture's
    /// green and the `PoC` probe's red so a CLAP `test_editor`
    /// surface is visually identifiable.
    const EDITOR_COLOR: u32 = 0x00_FF_00_00;

    /// This module's `HINSTANCE`, captured at registration — reused
    /// as the `hInstance` argument for `CreateWindowExW` so window
    /// creation is consistent with the class registration.
    static MODULE_H: AtomicIsize = AtomicIsize::new(0);

    /// Set once class registration succeeded (or the class was found
    /// already registered — including a stale copy left behind by a
    /// previous load of this module, which is safe precisely because
    /// the class's `wnd_proc` is `DefWindowProcW`).
    static REGISTERED: AtomicBool = AtomicBool::new(false);

    /// The wide class name — `"RuviformClapTestEditor"`, distinct
    /// from the VST3 fixture's `"RuviformTestEditor"` so both cdylibs
    /// can coexist in one host process without a class collision.
    fn class_name() -> Vec<u16> {
        "RuviformClapTestEditor"
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
    /// null, dead, or non-window handles — including opaque lease
    /// payloads that merely *look like* a handle — are rejected
    /// before any Win32 mutation.
    ///
    /// # Returns
    /// `Some` child HWND handle (`usize`) on success, `None` on any
    /// defined rejection — the caller maps `None` to a
    /// `PluginError`.
    pub(super) fn attach(parent: *mut c_void) -> Option<usize> {
        // SAFETY: `IsWindow` is a pure handle-validity query — safe on
        // any pointer value including null.
        if parent.is_null() || unsafe { IsWindow(parent) } == 0 {
            return None;
        }
        if !ensure_registered() {
            return None;
        }
        let class = class_name();
        // SAFETY: `parent` passed the `IsWindow` gate; the class is
        // registered; the returned HWND is owned by this gui state
        // until `destroy`.
        let child = unsafe {
            let mut rc = RECT::default();
            GetClientRect(parent, &raw mut rc);
            CreateWindowExW(
                0,
                class.as_ptr(),
                ptr::null(),
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

    /// Shows or hides the child HWND — `SW_SHOW`/`SW_HIDE` are
    /// idempotent window-state commands, safe on a live child.
    pub(super) fn set_visible(hwnd: usize, visible: bool) {
        // SAFETY: `hwnd` is the child this module created; the
        // command is a pure window-state change.
        unsafe {
            ShowWindow(hwnd as HWND, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    /// Resizes the child HWND to `width`×`height` pixels, keeping
    /// its position and z-order — the observable effect of `set_size`.
    /// Best-effort: a dead handle is a harmless `SetWindowPos` error.
    pub(super) fn resize(hwnd: usize, width: i32, height: i32) {
        // SAFETY: `hwnd` is the child this module created;
        // `SetWindowPos` on an already-dead handle is a defined
        // no-op error.
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

    /// Destroys the child HWND — `DestroyWindow` on an
    /// already-dead handle is a harmless error, so this is
    /// idempotent-safe.
    pub(super) fn destroy(hwnd: usize) {
        // SAFETY: `hwnd` is the child this module created; destroying
        // an already-dead handle is a defined no-op error.
        unsafe {
            DestroyWindow(hwnd as HWND);
        }
    }
}

/// Non-Windows shim — no HWND surface exists off Windows, so
/// `attach` always answers `None` (the caller's defined rejection)
/// and `destroy`/`set_visible`/`resize` are unreachable but defined.
#[cfg(not(target_os = "windows"))]
mod win32 {
    use std::ffi::c_void;

    pub(super) fn attach(_parent: *mut c_void) -> Option<usize> {
        None
    }

    pub(super) fn set_visible(_hwnd: usize, _visible: bool) {}

    pub(super) fn resize(_hwnd: usize, _width: i32, _height: i32) {}

    pub(super) fn destroy(_hwnd: usize) {}
}

// ---------------------------------------------------------------------------
// Audio processor
// ---------------------------------------------------------------------------

impl<'a> TestEditorAudioProcessor<'a> {
    /// The body of [`PluginAudioProcessor::activate`], minus the host
    /// handles the trait signature carries — kept as one authoritative
    /// body so tests drive the same `Activate`/`ActivateFailed`
    /// mapping the ABI callback does.
    ///
    /// **RT contract:** non-RT only — allocates per-block scratch.
    ///
    /// # Errors
    ///
    /// As [`PluginAudioProcessor::activate`]: config or preparation
    /// failure reports `ActivateFailed`; a lifecycle rejection is a
    /// host protocol violation.
    fn activate_inner(
        shared: &'a TestEditorShared,
        audio_config: &PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        let fail = |message| -> PluginError {
            let _ = shared.lifecycle.transition(LifecycleEvent::ActivateFailed);
            PluginError::Message(message)
        };
        let config = block::to_process_config(audio_config)
            .map_err(|_| fail("unsupported audio configuration"))?;
        let mut processor = TestEditorProcessor;
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
        Ok(Self {
            shared,
            processor,
            context,
            config,
            scratch: BlockScratch::new(block::MAX_CHANNELS, block::MAX_EVENTS_PER_BLOCK),
        })
    }
}

impl<'a> PluginAudioProcessor<'a, TestEditorShared, TestEditorMainThread<'a>>
    for TestEditorAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &TestEditorMainThread<'a>,
        shared: &'a TestEditorShared,
        audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Self::activate_inner(shared, &audio_config)
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ClapProcessStatus, PluginError> {
        // Lifecycle gate — `Process` is illegal outside `Processing`.
        self.shared
            .lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(|_| PluginError::Message("process outside processing state"))?;

        let frames_count = audio.frames_count();
        if frames_count > self.config.max_frames() {
            return Err(PluginError::Message(
                "frames_count exceeds negotiated maximum",
            ));
        }

        let mut views = match audio.port_pair(0) {
            Some(mut pair) => {
                block::build_block_views(&mut pair, &mut self.scratch, self.config.max_frames())?
            }
            None => BlockViews::empty(),
        };
        let mut output_list: [&mut [f32]; block::MAX_CHANNELS] = [&mut [], &mut []];
        let (input_view, output_view) = block::planar_views(&mut views, &mut output_list)?;

        // Every event kind is inert — the fixture declares no
        // parameter surface. The translator drops all events while
        // keeping the defined >MAX_EVENTS_PER_BLOCK rejection.
        let event_list =
            block::translate_events(events.input, &mut self.scratch, frames_count, |_| None)?;
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result =
            self.processor
                .process(&inputs, &mut outputs, event_list, None, &mut self.context);
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestEditorMainThread<'a>) {
        let _ = self.shared.lifecycle.transition(LifecycleEvent::Deactivate);
    }

    fn reset(&mut self) {
        // Stateless pass-through — nothing to resync.
    }

    fn start_processing(&mut self) -> Result<(), PluginError> {
        self.shared
            .lifecycle
            .transition(LifecycleEvent::StartProcessing)
            .map_err(|_| PluginError::Message("start_processing outside Activated state"))?;
        Ok(())
    }

    fn stop_processing(&mut self) {
        let _ = self
            .shared
            .lifecycle
            .transition(LifecycleEvent::StopProcessing);
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::ProcessStatus;

    /// A fresh main-thread object — the gui state is self-contained,
    /// so the `PhantomData` shared marker needs no live `Shared`.
    fn main_thread() -> TestEditorMainThread<'static> {
        TestEditorMainThread {
            gui: RefCell::new(GuiState::default()),
            _shared: PhantomData,
        }
    }

    /// `is_api_supported` answers `true` only for `win32` + embedded
    /// on Windows builds — floating windows, every other API type,
    /// and non-Windows builds are honest `false`.
    #[test]
    fn api_support_is_win32_embedded_on_windows_only() {
        let main = main_thread();
        assert_eq!(
            main.is_api_supported(EMBEDDED_WIN32),
            cfg!(windows),
            "win32+embedded is the only supported configuration"
        );
        let floating = GuiConfiguration {
            api_type: GuiApiType::WIN32,
            is_floating: true,
        };
        assert!(!main.is_api_supported(floating), "floating is unsupported");
        let cocoa = GuiConfiguration {
            api_type: GuiApiType::COCOA,
            is_floating: false,
        };
        assert!(
            !main.is_api_supported(cocoa),
            "non-win32 api is unsupported"
        );
        let custom = GuiConfiguration {
            api_type: GuiApiType(c"custom"),
            is_floating: false,
        };
        assert!(!main.is_api_supported(custom), "custom api is unsupported");
        assert_eq!(
            main.get_preferred_api(),
            cfg!(windows).then_some(EMBEDDED_WIN32),
            "preferred api mirrors the supported set"
        );
    }

    /// Size verbs are honest: `get_size` reports the current size
    /// (640x480 until `set_size`), `can_resize` accepts, `set_size`
    /// records the requested size and resizes the attached child —
    /// `adjust_size`/`get_resize_hints` keep the no-constraint
    /// defaults, `set_size` before `create` is a defined rejection,
    /// `set_scale` accepts only finite positive factors, and
    /// `set_transient` always rejects (floating is unsupported).
    #[test]
    fn size_and_transient_verbs_are_honest() {
        let main = main_thread();
        assert_eq!(main.get_size(), Some(DEFAULT_GUI_SIZE));
        assert!(main.can_resize());
        assert_eq!(main.get_resize_hints(), None);
        assert_eq!(main.adjust_size(DEFAULT_GUI_SIZE), None);
        // `set_size` before `create` is a protocol violation on every
        // platform — and off Windows `create` itself declines, so the
        // verb stays a defined rejection there too.
        assert!(main.set_size(DEFAULT_GUI_SIZE).is_err());
        assert!(main.set_scale(1.0).is_ok());
        assert!(main.set_scale(2.5).is_ok());
        assert!(main.set_scale(0.0).is_err());
        assert!(main.set_scale(-1.0).is_err());
        assert!(main.set_scale(f64::NAN).is_err());
        assert!(main.set_scale(f64::INFINITY).is_err());
        let transient = Window::from_generic_ptr(GuiApiType::WIN32, ptr::null_mut());
        assert!(main.set_transient(transient).is_err());
        // On Windows, `create` → `set_size` records the size
        // `get_size` echoes and resizes the attached child.
        #[cfg(target_os = "windows")]
        {
            assert!(main.create(EMBEDDED_WIN32).is_ok());
            let bigger = GuiSize {
                width: 800,
                height: 600,
            };
            assert!(main.set_size(bigger).is_ok());
            assert_eq!(main.get_size(), Some(bigger));
            main.destroy();
        }
    }

    /// `create` validates the negotiated configuration and records
    /// the lifecycle edge; a second `create` is a defined rejection
    /// and `destroy` is idempotent — including after a failed
    /// `set_parent`/`show` rollback.
    #[test]
    fn create_and_destroy_are_idempotent_edges() {
        let main = main_thread();
        // Unsupported configuration — defined rejection.
        let floating = GuiConfiguration {
            api_type: GuiApiType::WIN32,
            is_floating: true,
        };
        assert!(main.create(floating).is_err());
        // `create` with the supported config succeeds only where the
        // build can actually create a window.
        if cfg!(windows) {
            assert!(main.create(EMBEDDED_WIN32).is_ok());
            assert!(main.create(EMBEDDED_WIN32).is_err(), "double create");
            main.destroy();
            main.destroy();
            // Re-create after destroy is legal.
            assert!(main.create(EMBEDDED_WIN32).is_ok());
            main.destroy();
        } else {
            assert!(main.create(EMBEDDED_WIN32).is_err());
        }
    }

    /// `set_parent` rejects honestly on every platform: a non-Win32
    /// `Window`, a null HWND payload, and a payload that is not a
    /// live window all fail before any child HWND exists — and
    /// `set_parent` before `create` is a protocol violation.
    #[test]
    fn set_parent_rejects_invalid_parents_honestly() {
        let main = main_thread();
        let cocoa = Window::from_generic_ptr(GuiApiType(c"cocoa"), ptr::null_mut());
        assert!(
            main.set_parent(cocoa).is_err(),
            "non-win32 window must be rejected"
        );
        let null_hwnd = Window::from_generic_ptr(GuiApiType::WIN32, ptr::null_mut());
        assert!(main.set_parent(null_hwnd).is_err());
        let bogus = Window::from_generic_ptr(
            GuiApiType::WIN32,
            0xDEAD_BEEF_usize as *mut core::ffi::c_void,
        );
        assert!(
            main.set_parent(bogus).is_err(),
            "a payload that is not a live window must be rejected"
        );
        // On Windows, drive the full create → reject → rollback path:
        // `set_parent` with a bad payload must leave `created` true
        // (the host's `destroy` rollback then resets it) and no child
        // behind.
        #[cfg(target_os = "windows")]
        {
            assert!(main.create(EMBEDDED_WIN32).is_ok());
            assert!(main.set_parent(bogus).is_err());
            assert!(main.set_parent(null_hwnd).is_err());
            assert!(main.gui.borrow().hwnd.is_none());
            main.destroy();
        }
    }

    /// `show`/`hide` require an attached parent — calling them before
    /// `set_parent` (or before `create`) is a defined rejection, not
    /// a fabricated success.
    #[test]
    fn show_and_hide_require_an_attached_parent() {
        let main = main_thread();
        assert!(main.show().is_err());
        assert!(main.hide().is_err());
    }

    /// The DSP law at the neutral-contract layer: one block copies
    /// `in` to `out` bitwise and reports `Continue`; empty buses
    /// report `Sleep`.
    #[test]
    fn processor_is_bitwise_pass_through() {
        let mut processor = TestEditorProcessor;
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let frames = u32::try_from(input[0].len()).unwrap();
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(&[], frames).unwrap();
        let mut context = ProcessContext::new();
        let result = processor.process(&inputs, &mut outputs, list, None, &mut context);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);

        let empty_in: [PlanarBufferView<'_>; 0] = [];
        let mut empty_out: [PlanarBufferViewMut<'_>; 0] = [];
        let list = EventList::new(&[], frames).unwrap();
        let result = processor.process(&empty_in, &mut empty_out, list, None, &mut context);
        assert_eq!(result.status, ProcessStatus::Sleep);
    }

    /// Windows-only: `create` → `set_parent` against a real parent
    /// HWND creates a real `WS_CHILD` window — the opt-in evidence
    /// path the `scarecrow173/ruviform#631` real-attach lane
    /// formalizes. `#[ignore]` keeps it out of the default lane (it
    /// needs a window station); run explicitly with
    /// `cargo test -p ruviform-plugin-test-clap -- --ignored`.
    #[cfg(target_os = "windows")]
    #[ignore = "requires a Windows window station — real-attach lane owns the formal evidence"]
    #[expect(unsafe_code)]
    #[test]
    fn set_parent_creates_real_child_hwnd() {
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

        let main = main_thread();
        assert!(main.create(EMBEDDED_WIN32).is_ok());
        // SAFETY: `parent` is a live HWND owned by this thread for
        // the duration of the test.
        let window = unsafe { Window::from_win32_hwnd(parent) };
        assert!(main.set_parent(window).is_ok());
        assert!(main.show().is_ok());

        unsafe {
            // The child is observable through Win32 directly: the
            // parent must now have a child, parented correctly and
            // alive.
            let child: HWND = GetWindow(parent, GW_CHILD);
            assert!(!child.is_null(), "set_parent must create a child HWND");
            assert_eq!(GetParent(child), parent, "child must be parented");
            assert_ne!(IsWindow(child), 0, "child must be a live window");
            // A second `set_parent` without `destroy` is a defined
            // rejection, not a second window.
            assert!(main.set_parent(window).is_err());
            assert!(main.hide().is_ok());
            main.destroy();
            assert_eq!(IsWindow(child), 0, "destroy must destroy the child");
            let _ = DestroyWindow(parent);
        }
    }
}
