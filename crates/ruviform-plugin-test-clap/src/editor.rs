//! The `test_editor` CLAP plugin: the suite's `clap.ext.gui` editor
//! fixture carrying the shared `Fault` parameter — the CLAP half of
//! Issue #630 AC-4 ("Native Editor が実 child HWND を生成できる"),
//! mirroring the VST3 `test_editor` fixture
//! (`ruviform-plugin-test-vst3`'s `editor.rs`/`editor_view.rs`,
//! whose `test_editor` gained the same `Fault` arm in Issue #885).
//!
//! ## DSP/param law
//!
//! `test_editor` declares the [`FAULT_PARAM`] descriptor (`ParamId`
//! `0`, plain `[0.0, 8.0]`, stepped — eight modes) through the
//! `params` extension, so in-band `ParamValue` events pass
//! `block::translate_param_event` and arm the fault inside the
//! arming block's `process()`. Unarmed (`none`) the class is pure
//! pass-through (`out == in` bitwise). The fault law itself is
//! [`crate::fault`]'s single source of truth, consumed here through
//! [`TestEditorProcessor`]'s delegation to [`TestFaultProcessor`] —
//! the same reuse the VST3 `TestEditorProcessor(TestFaultProcessor)`
//! applies. `abort`/`exit`/`hang` kill or wedge the *host process*:
//! they exist for the L1 out-of-process child the
//! `real-clap-session-crash`/`real-clap-session-hang` lane spawns,
//! never in-process.
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
//!   `win32` on Windows builds — embedded *and* floating (the
//!   plugin-owned popup surface); other APIs and non-Windows builds
//!   are honest `false`/`None`;
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
//!   defined rejection, `set_scale` acknowledges a finite positive
//!   factor (CLAP permits ignoring it — the Win32 child uses
//!   physical pixels);
//! - floating mode is the fixture's popup surface: `set_transient`
//!   records the host window the popup stays above (embedded mode
//!   is the honest refusal), `suggest_title` names the popup HWND,
//!   and `show` creates a real `WS_POPUP` window owned by the
//!   transient parent on first call — `set_parent` on a floating
//!   GUI is the defined rejection in the opposite direction.
//! - `ruviform.editor-ops` is the private extension seam (Issue
//!   #630 K1) the host adapter's `send_gui_key`/`set_gui_focus`/
//!   `set_gui_focus_target`/`set_gui_popup` deliver through —
//!   `clap.ext.gui` carries no key/focus/popup verbs, so a
//!   ruviform-namespaced `#[repr(C)]` vtable (`deliver_key`,
//!   `apply_focus`, `popup`) is registered next to `PluginGui`.
//!   Delivery is recorded as event-log tokens in the child HWND's
//!   window text (`rvf-test-editor-attach-{n}|{ev},{ev},…` — the
//!   VST3 `window_title` grammar verbatim: `f:`/`fe:`/`kd:`/`ku:`/
//!   `po:`/`pc:`), synthetic verb delivery only: no OS input queue
//!   or focus routing is exercised. `apply_focus(Editor, 1)` also
//!   runs a real `SetFocus` on the child so the host can read
//!   `GetFocus() == child`; the `Dialog`-kind popup is a real
//!   `WS_POPUP` owned by the child's root ancestor and the
//!   `ChildWindow`-kind a real `WS_CHILD` under it, both titled
//!   `rvf-test-editor-popup-{seq}`.
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

use core::fmt::Write as _;
use std::cell::RefCell;
use std::ffi::CStr;

use clack_common::events::io::{InputEvents, OutputEvents};
use clack_common::plugin::features;
use clack_common::utils::{ClapId, Cookie};
use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_extensions::gui::{
    GuiApiType, GuiConfiguration, GuiSize, PluginGui, PluginGuiImpl, Window,
};
use clack_extensions::params::{
    ParamDisplayWriter, ParamInfoFlags, ParamInfoWriter, PluginAudioProcessorParams,
    PluginMainThreadParams, PluginParams,
};
use clack_plugin::extensions::PluginExtensions;
use clack_plugin::extensions::prelude::{
    Extension, ExtensionImplementation, PluginExtensionSide, PluginWrapper, RawExtension,
    RawExtensionImplementation, clap_plugin,
};
use clack_plugin::host::{HostAudioProcessorHandle, HostMainThreadHandle, HostSharedHandle};
use clack_plugin::plugin::{
    Plugin, PluginAudioProcessor, PluginDescriptor, PluginError, PluginMainThread, PluginShared,
};
use clack_plugin::process::{
    Audio, Events, PluginAudioConfiguration, Process, ProcessStatus as ClapProcessStatus,
};

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, LifecycleEvent, PluginProcessor, PrepareError,
    ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

use crate::block::{self, AtomicLifecycle, BlockScratch, BlockViews, ParamMirror};
use crate::fault::{
    FAULT_PARAM, FaultMode, TestFaultProcessor, bad_time_output_event, decode_fault,
    invalid_output_event,
};

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
    /// The negotiated mode from `create`: `true` = the CLAP floating
    /// surface (plugin-owned popup window), `false` = embedded.
    floating: bool,
    /// The embedded child HWND — or, in floating mode, the
    /// plugin-owned popup HWND (`None` until `set_parent`/`show`).
    /// Only ever `Some` on Windows — the non-Windows `win32` shim
    /// never produces one.
    hwnd: Option<usize>,
    /// Floating mode only: the transient parent `set_transient`
    /// handed the plugin — the window the popup stays above.
    transient: Option<usize>,
    /// Floating mode only: the title `suggest_title` recorded —
    /// applied to the popup HWND's window text when it is created.
    title: Option<String>,
    /// The current editor size — `get_size` reports it, `set_size`
    /// replaces it (and resizes the child HWND when attached).
    size: GuiSize,
    /// Attach generation — stamped into the child HWND's window text
    /// (`rvf-test-editor-attach-{n}`) so an observer can prove a
    /// `destroy` + `create`/`set_parent` cycle really recreated the
    /// surface (the VST3 fixture's `attach_seq` oracle).
    attach_seq: u32,
    /// The recorded `ruviform.editor-ops` verb tokens —
    /// `f:{state}`, `fe:{element}:{state}`,
    /// `kd:{key:04x}:{key_code:04x}:{modifiers:04x}`, `ku:{…}`,
    /// `po:{seq}`/`pc:{seq}` — appended to the child HWND's window
    /// text so an observer can prove the verbs reached the plugin
    /// through the same `GetWindowTextW` channel the attach
    /// generation uses (synthetic verb delivery — no OS input queue
    /// or focus routing is claimed). Cleared per attach generation.
    events: Vec<String>,
    /// The `ruviform.editor-ops` popup HWND (`ChildWindow` or
    /// `Dialog` kind) — HWND-handle only: `Some` only when this
    /// build produced a window, never `Some` off Windows or after a
    /// creation failure. The open/close oracle is
    /// `verb_popup_open`, not this field.
    verb_popup: Option<usize>,
    /// The logical popup open state — `true` after an open verb,
    /// `false` after a close verb or `destroy`, whether or not
    /// `verb_popup` holds a real HWND. Off Windows the flag still
    /// toggles so `po:`/`pc:` is recorded on every OS (the VST3
    /// `popup_open` precedent — logical and HWND faces separate).
    verb_popup_open: bool,
    /// Popup generation — stamped into the popup HWND's title
    /// (`rvf-test-editor-popup-{n}`) and the `po:`/`pc:` tokens so
    /// an observer can prove a close/open cycle really recreated
    /// the surface.
    verb_popup_seq: u32,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            created: false,
            floating: false,
            hwnd: None,
            transient: None,
            title: None,
            size: DEFAULT_GUI_SIZE,
            attach_seq: 0,
            events: Vec::new(),
            verb_popup: None,
            verb_popup_open: false,
            verb_popup_seq: 0,
        }
    }
}

impl GuiState {
    /// The child HWND's window text: the attach generation tag plus
    /// the recorded verb log — `rvf-test-editor-attach-{n}
    /// [|{ev},{ev},…]` (the VST3 `window_title` grammar verbatim —
    /// identical token text so observers share one parser across
    /// ABIs).
    fn window_title(&self) -> String {
        let mut title = format!("rvf-test-editor-attach-{}", self.attach_seq);
        if !self.events.is_empty() {
            title.push('|');
            title.push_str(&self.events.join(","));
        }
        title
    }

    /// Records one verb token and restamps the child HWND's window
    /// text — the fixture's delivery-evidence channel (synthetic
    /// verb delivery; no OS input queue or focus routing claim).
    fn record(&mut self, token: String) {
        self.events.push(token);
        if let Some(hwnd) = self.hwnd {
            win32::set_title(hwnd, &self.window_title());
        }
    }
}

// ---------------------------------------------------------------------------
// Processor (neutral contract — no clack types)
// ---------------------------------------------------------------------------

/// The `test_editor` processor — the [`TestFaultProcessor`] law
/// (pass-through DSP + armed-fault dispatch) behind the editor
/// fixture's own type, the same delegation the VST3
/// `TestEditorProcessor(TestFaultProcessor)` applies (Issue #885).
pub(crate) struct TestEditorProcessor(TestFaultProcessor);

impl TestEditorProcessor {
    /// A fresh processor at the parameter default (`armed = 0.0` —
    /// fully inert).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(TestFaultProcessor::new())
    }

    /// The current armed plain value — the mirror's sync-out source.
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn armed_plain(&self) -> f64 {
        self.0.armed_plain()
    }

    /// Sets the armed plain value — the mirror's sync-in target
    /// (activation reset, `reset`, main/audio-thread `flush`).
    ///
    /// **RT contract:** RT-suitable — pure.
    pub(crate) fn set_armed_plain(&mut self, plain: f64) {
        self.0.set_armed_plain(plain);
    }
}

impl PluginProcessor for TestEditorProcessor {
    /// Activation hook — resets the armed value to the parameter
    /// default (the delegated [`TestFaultProcessor::prepare`] law).
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.0.prepare(config, context)
    }

    /// Runs one block through the delegated [`TestFaultProcessor`]
    /// law: pass-through copy, then the armed-value update from the
    /// block's `Fault` events, then the mode dispatch.
    ///
    /// **RT contract:** the `none` path is RT-suitable — bounded
    /// `O(frames × channels + events)` work; no allocation, locking,
    /// waiting or I/O. The armed paths intentionally violate RT
    /// rules (that is the fault under test).
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        self.0.process(inputs, outputs, events, transport, context)
    }
}

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

/// Plugin-wide, thread-safe state (`[thread-safe]` in the CLAP model):
/// the shared lifecycle machine and the fault parameter mirror.
pub(crate) struct TestEditorShared {
    /// The neutral lifecycle state machine, shared across threads.
    lifecycle: AtomicLifecycle,
    /// The fault parameter's current plain (armed) value, mirrored
    /// for the main-thread `params` extension.
    fault_mirror: ParamMirror,
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
            fault_mirror: ParamMirror::new(FAULT_PARAM.default_value),
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
/// metadata, the `params` extension surface, and the `clap.ext.gui`
/// state live here — the gui verbs are self-contained in
/// [`GuiState`], the params verbs read the shared fault mirror.
pub(crate) struct TestEditorMainThread<'a> {
    gui: RefCell<GuiState>,
    shared: &'a TestEditorShared,
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
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>()
            .register::<PluginGui>()
            .register::<RuviformEditorOps>();
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
        shared: &'a TestEditorShared,
    ) -> Result<TestEditorMainThread<'a>, PluginError> {
        Ok(TestEditorMainThread {
            gui: RefCell::new(GuiState::default()),
            shared,
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
// Params extension
// ---------------------------------------------------------------------------

impl PluginMainThreadParams for TestEditorMainThread<'_> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }
        info.set(&clack_extensions::params::ParamInfo {
            id: ClapId::new(FAULT_PARAM.id.0),
            flags: ParamInfoFlags::IS_AUTOMATABLE
                | ParamInfoFlags::IS_STEPPED
                | ParamInfoFlags::REQUIRES_PROCESS,
            cookie: Cookie::empty(),
            name: FAULT_PARAM.name.as_bytes(),
            module: b"",
            min_value: FAULT_PARAM.min_value,
            max_value: FAULT_PARAM.max_value,
            default_value: FAULT_PARAM.default_value,
        });
    }

    fn get_value(&self, param_id: ClapId) -> Option<f64> {
        (param_id.get() == FAULT_PARAM.id.0).then(|| self.shared.fault_mirror.load())
    }

    fn value_to_text(
        &self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        if param_id.get() != FAULT_PARAM.id.0 {
            return Err(core::fmt::Error);
        }
        write!(writer, "{value:.2}")
    }

    fn text_to_value(&self, param_id: ClapId, text: &CStr) -> Option<f64> {
        if param_id.get() != FAULT_PARAM.id.0 {
            return None;
        }
        let text = text.to_str().ok()?.trim();
        let value: f64 = text.parse().ok()?;
        value
            .is_finite()
            .then(|| value.clamp(FAULT_PARAM.min_value, FAULT_PARAM.max_value))
    }

    fn flush(&self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Main-thread flush: fold every ParamValue for our param into
        // the mirror; offsets carry no meaning outside a process
        // block. The scan is bounded like the process path's.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &FAULT_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .fault_mirror
                    .store(FAULT_PARAM.denormalize(param_event.value));
            }
        }
    }
}

impl PluginAudioProcessorParams for TestEditorAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _output: &mut OutputEvents) {
        // Audio-thread flush: mirror the change and resync the live
        // processor — CLAP does not guarantee offsets here.
        for unknown in input_parameter_changes
            .iter()
            .take(block::MAX_EVENTS_PER_BLOCK)
        {
            if let Some(event) = block::translate_param_event(unknown, &FAULT_PARAM)
                && let EventKind::ParamValue(param_event) = event.kind()
            {
                self.shared
                    .fault_mirror
                    .store(FAULT_PARAM.denormalize(param_event.value));
            }
        }
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
    }
}

// ---------------------------------------------------------------------------
// GUI extension (`clap.ext.gui`, plugin side)
// ---------------------------------------------------------------------------

impl PluginGuiImpl for TestEditorMainThread<'_> {
    /// `win32` on Windows builds — embedded *and* floating (the
    /// plugin-owned popup surface): other windowing APIs and
    /// non-Windows builds are honest `false`. The host bridge's
    /// `open_gui` requires exactly this gate before it calls
    /// `create` (embedded); `create` is the negotiation point that
    /// records which mode the session runs.
    fn is_api_supported(&self, configuration: GuiConfiguration) -> bool {
        configuration.api_type == GuiApiType::WIN32 && cfg!(windows)
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
        state.floating = configuration.is_floating;
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
        if let Some(popup) = state.verb_popup.take() {
            win32::destroy(popup);
        }
        state.verb_popup_open = false;
        state.created = false;
        state.floating = false;
        state.transient = None;
        state.title = None;
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
        if state.floating {
            // A floating surface has no parent to embed into —
            // honest refusal, not a faked embed.
            return Err(PluginError::Message("set_parent on floating gui"));
        }
        if state.hwnd.is_some() {
            return Err(PluginError::Message("gui already embedded"));
        }
        // A fresh attach generation: the sequence number stamps the
        // child's title (`rvf-test-editor-attach-{n}`) and the verb
        // event log resets with it — a recreated surface reads as a
        // fresh generation, never inheriting the last one's tokens
        // (the VST3 `attached` precedent).
        state.attach_seq = state.attach_seq.wrapping_add(1);
        state.events.clear();
        let title = state.window_title();
        let child = win32::attach(parent, &title).ok_or(PluginError::Message(
            "set_parent: child window creation failed",
        ))?;
        state.hwnd = Some(child);
        Ok(())
    }

    /// Floating mode: records the window the plugin-owned popup
    /// should stay above — the CLAP popup surface's host half. The
    /// supplied [`Window`] must be a Win32 window whose handle is a
    /// live window (same `IsWindow` gate `set_parent` applies).
    /// Embedded mode is the honest refusal the popup verb gets on a
    /// surface that cannot be transient.
    fn set_transient(&self, window: Window) -> Result<(), PluginError> {
        let parent = window
            .as_win32_hwnd()
            .ok_or(PluginError::Message("set_transient: not a win32 window"))?;
        let mut state = self
            .gui
            .try_borrow_mut()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        if !state.created {
            return Err(PluginError::Message("set_transient before create"));
        }
        if !state.floating {
            return Err(PluginError::Message("set_transient on embedded gui"));
        }
        if !win32::is_window(parent as usize) {
            return Err(PluginError::Message("set_transient: not a live window"));
        }
        state.transient = Some(parent as usize);
        Ok(())
    }

    /// Floating mode: records the host-suggested title — applied to
    /// the popup HWND's window text when `show` creates it, and to a
    /// live popup directly. Embedded mode acknowledges and ignores
    /// (CLAP permits ignoring a suggestion that names no surface).
    fn suggest_title(&self, title: &str) {
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return;
        };
        state.title = Some(title.to_string());
        if let Some(hwnd) = state.hwnd {
            win32::set_title(hwnd, title);
        }
    }

    /// Shows the surface: embedded mode flips the child `SW_SHOW`;
    /// floating mode creates the plugin-owned popup HWND on first
    /// `show` (a `WS_POPUP` owned by the recorded transient parent,
    /// titled `rvf-test-editor-popup` or the host-suggested title)
    /// and shows it. `show` before `create` — or on embedded before
    /// `set_parent` — is a host protocol violation and a defined
    /// rejection.
    fn show(&self) -> Result<(), PluginError> {
        let mut state = self
            .gui
            .try_borrow_mut()
            .map_err(|_| PluginError::Message("gui state re-entered"))?;
        if state.floating && state.hwnd.is_none() {
            let title = state
                .title
                .clone()
                .unwrap_or_else(|| "rvf-test-editor-popup".to_string());
            let hwnd = win32::popup(state.transient, &title, state.size)
                .ok_or(PluginError::Message("popup window creation failed"))?;
            state.hwnd = Some(hwnd);
        }
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
// Private editor-ops extension (`ruviform.editor-ops`, plugin side)
// ---------------------------------------------------------------------------

/// `GuiKeyDirection::Down`'s `ruviform.editor-ops` discriminant —
/// the wire-v8 value both sides agree on.
const EDITOR_OPS_KEY_DOWN: u8 = 0;
/// `GuiKeyDirection::Up`'s `ruviform.editor-ops` discriminant.
const EDITOR_OPS_KEY_UP: u8 = 1;
/// `GuiFocusTarget::Editor`'s `ruviform.editor-ops` discriminant.
const EDITOR_OPS_TARGET_EDITOR: u8 = 0;
/// `GuiFocusTarget::EditorElement`'s `ruviform.editor-ops`
/// discriminant.
const EDITOR_OPS_TARGET_ELEMENT: u8 = 1;
/// `GuiPopupKind::ChildWindow`'s `ruviform.editor-ops` discriminant.
const EDITOR_OPS_POPUP_CHILD: u8 = 0;
/// `GuiPopupKind::Dialog`'s `ruviform.editor-ops` discriminant.
const EDITOR_OPS_POPUP_DIALOG: u8 = 1;

/// The verb-driven popup's size — the same 200×100 the VST3
/// fixture's dialog HWND gets, so the top-level inventory oracle
/// reads the same geometry across ABIs.
const VERB_POPUP_SIZE: GuiSize = GuiSize {
    width: 200,
    height: 100,
};

/// The `#[repr(C)]` vtable the fixture hands `plugin.get_extension`
/// for the private `ruviform.editor-ops` id — the fixture-private
/// seam (Issue #630 K1) through which the host adapter's
/// `send_gui_key`/`set_gui_focus`/`set_gui_focus_target`/
/// `set_gui_popup` reach this plugin, `clap.ext.gui` carrying no
/// key/focus/popup verbs of its own.
///
/// **ABI contract — mirrored verbatim by the adapter**
/// (`ruviform-plugin-clap` `host.rs`; the two crates cannot share
/// the type, so this layout plus the discriminant consts above is
/// the contract):
///
/// - extension id: `ruviform.editor-ops`
/// - field order: `deliver_key`, `apply_focus`, `popup`
/// - `deliver_key(plugin, direction, key, key_code, modifiers) ->
///   bool` — `direction` is `GuiKeyDirection`'s discriminant
///   (`Down` = 0, `Up` = 1); `key` the UTF-16 code unit; `key_code`
///   the Win32 virtual-key value; `modifiers` the `GuiModifiers`
///   bitmask — all 16-bit values in `u32` slots.
/// - `apply_focus(plugin, focused, target, element) -> bool` —
///   `focused` is strict `{0,1}`; `target` is `GuiFocusTarget`'s
///   discriminant (`Editor` = 0, `EditorElement` = 1; `HostWindow`
///   = 2 never reaches the plugin — the adapter consumes it);
///   `element` the opaque id, `0` for non-element targets.
/// - `popup(plugin, kind, open) -> bool` — `kind` is
///   `GuiPopupKind`'s discriminant (`ChildWindow` = 0, `Dialog` = 1);
///   `open` is strict `{0,1}`.
/// - every member returns `false` on a defined rejection
///   (unassigned byte, out-of-range value, no created GUI) — the
///   adapter maps `false` to `HostError::PluginFailed`.
#[repr(C)]
#[derive(Copy, Clone)]
struct ClapPluginRuviformEditorOps {
    deliver_key: Option<
        unsafe extern "C" fn(
            plugin: *const clap_plugin,
            direction: u8,
            key: u32,
            key_code: u32,
            modifiers: u32,
        ) -> bool,
    >,
    apply_focus: Option<
        unsafe extern "C" fn(
            plugin: *const clap_plugin,
            focused: u8,
            target: u8,
            element: u32,
        ) -> bool,
    >,
    popup: Option<unsafe extern "C" fn(plugin: *const clap_plugin, kind: u8, open: u8) -> bool>,
}

/// The extension type `declare_extensions` registers — the
/// [`ExtensionImplementation`] below supplies the vtable. The
/// plugin side never wraps an incoming pointer (the host does), so
/// this is a unit type whose `from_raw` is never reached.
#[derive(Copy, Clone)]
struct RuviformEditorOps;

// SAFETY: `IDENTIFIERS` names the private seam whose vtable layout
// `ClapPluginRuviformEditorOps` pins down. `from_raw` is a no-op
// stub — the plugin side only ever *produces* the extension
// pointer (`ExtensionImplementation`), never consumes one.
#[expect(unsafe_code)]
unsafe impl Extension for RuviformEditorOps {
    const IDENTIFIERS: &[&CStr] = &[c"ruviform.editor-ops"];
    type ExtensionSide = PluginExtensionSide;

    unsafe fn from_raw(_raw: RawExtension<Self::ExtensionSide>) -> Self {
        Self
    }
}

/// The plugin-side `ruviform.editor-ops` surface — the three verbs
/// the seam delivers to the main-thread object. Every method
/// answers `false` on a defined rejection (re-entrant state, no
/// created GUI, unassigned byte, out-of-range value, a repeated
/// edge); the adapter maps `false` to `HostError::PluginFailed`.
trait RuviformEditorOpsImpl {
    /// One key event — `direction` is the `GuiKeyDirection`
    /// discriminant, `key` the UTF-16 code unit, `key_code` the
    /// Win32 virtual-key value, `modifiers` the `GuiModifiers`
    /// bitmask.
    fn deliver_key(&self, direction: u8, key: u32, key_code: u32, modifiers: u32) -> bool;
    /// One focus verb — `focused` strict `{0,1}`, `target` the
    /// `GuiFocusTarget` discriminant, `element` the opaque id.
    fn apply_focus(&self, focused: u8, target: u8, element: u32) -> bool;
    /// One popup verb — `kind` the `GuiPopupKind` discriminant,
    /// `open` strict `{0,1}`.
    fn popup(&self, kind: u8, open: u8) -> bool;
}

/// The `deliver_key` vtable entry — routes the ABI call to the
/// plugin's main-thread object. `PluginWrapper::handle` answers
/// `None` on a null plugin or a wrapper error → `false`, the
/// seam's defined rejection.
///
/// # Safety
/// `plugin` must be a live `clap_plugin` of this plugin type — the
/// `get_extension` → vtable contract guarantees it, and `handle`
/// re-validates the wrapper before touching it.
#[expect(unsafe_code)]
unsafe extern "C" fn editor_ops_deliver_key<P: Plugin>(
    plugin: *const clap_plugin,
    direction: u8,
    key: u32,
    key_code: u32,
    modifiers: u32,
) -> bool
where
    for<'a> P::MainThread<'a>: RuviformEditorOpsImpl,
{
    // SAFETY: upheld by the caller contract — `handle` only touches
    // the wrapper when `plugin` names a live instance of `P`, and
    // `main_thread` is only reached on the plugin's main thread
    // (the host calls GUI verbs on the parent HWND's pump thread).
    unsafe {
        PluginWrapper::<P>::handle(plugin, |p| {
            Ok(p.main_thread()
                .deliver_key(direction, key, key_code, modifiers))
        })
        .unwrap_or(false)
    }
}

/// The `apply_focus` vtable entry — same routing contract as
/// [`editor_ops_deliver_key`].
///
/// # Safety
/// Same contract as [`editor_ops_deliver_key`].
#[expect(unsafe_code)]
unsafe extern "C" fn editor_ops_apply_focus<P: Plugin>(
    plugin: *const clap_plugin,
    focused: u8,
    target: u8,
    element: u32,
) -> bool
where
    for<'a> P::MainThread<'a>: RuviformEditorOpsImpl,
{
    // SAFETY: same contract as `editor_ops_deliver_key`.
    unsafe {
        PluginWrapper::<P>::handle(plugin, |p| {
            Ok(p.main_thread().apply_focus(focused, target, element))
        })
        .unwrap_or(false)
    }
}

/// The `popup` vtable entry — same routing contract as
/// [`editor_ops_deliver_key`].
///
/// # Safety
/// Same contract as [`editor_ops_deliver_key`].
#[expect(unsafe_code)]
unsafe extern "C" fn editor_ops_popup<P: Plugin>(
    plugin: *const clap_plugin,
    kind: u8,
    open: u8,
) -> bool
where
    for<'a> P::MainThread<'a>: RuviformEditorOpsImpl,
{
    // SAFETY: same contract as `editor_ops_deliver_key`.
    unsafe {
        PluginWrapper::<P>::handle(plugin, |p| Ok(p.main_thread().popup(kind, open)))
            .unwrap_or(false)
    }
}

// SAFETY: `IMPLEMENTATION` points at the `#[repr(C)]` static
// `ClapPluginRuviformEditorOps` vtable — the documented
// `ruviform.editor-ops` ABI contract; the three slots are
// instantiated for `P` under the same bound `register::<
// RuviformEditorOps>` requires.
#[expect(unsafe_code)]
unsafe impl<P: Plugin> ExtensionImplementation<P> for RuviformEditorOps
where
    for<'a> P::MainThread<'a>: RuviformEditorOpsImpl,
{
    const IMPLEMENTATION: RawExtensionImplementation =
        RawExtensionImplementation::new(&ClapPluginRuviformEditorOps {
            deliver_key: Some(editor_ops_deliver_key::<P>),
            apply_focus: Some(editor_ops_apply_focus::<P>),
            popup: Some(editor_ops_popup::<P>),
        });
}

impl RuviformEditorOpsImpl for TestEditorMainThread<'_> {
    /// `deliver_key` — records `kd:`/`ku:` into the event log and
    /// restamps the child title (synthetic verb delivery — no OS
    /// input queue is exercised). Defined rejections: a re-entrant
    /// state borrow, no created GUI, an unassigned `direction`, or
    /// a field wider than the 16-bit contract slots.
    fn deliver_key(&self, direction: u8, key: u32, key_code: u32, modifiers: u32) -> bool {
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return false;
        };
        if !state.created {
            return false;
        }
        let prefix = match direction {
            EDITOR_OPS_KEY_DOWN => "kd",
            EDITOR_OPS_KEY_UP => "ku",
            _ => return false,
        };
        if key > u32::from(u16::MAX)
            || key_code > u32::from(u16::MAX)
            || modifiers > u32::from(u16::MAX)
        {
            return false;
        }
        state.record(format!("{prefix}:{key:04x}:{key_code:04x}:{modifiers:04x}"));
        true
    }

    /// `apply_focus` — records `f:{0|1}` for `Editor` and
    /// `fe:{element}:{0|1}` for `EditorElement` (synthetic delivery;
    /// `EditorElement` records intent only). On Windows an
    /// `Editor`-target `focused=1` also runs a real `SetFocus` on
    /// the child HWND — the host-side `GetFocus() == child` oracle
    /// (callers return focus through `HostWindow`/`false` in the
    /// same case, so the pair stays one sequence). Defined
    /// rejections: strict-flag violation, a re-entrant borrow, no
    /// created GUI, `element != 0` on a non-element target, or an
    /// unassigned `target` (`HostWindow` never reaches the plugin —
    /// the adapter consumes it).
    fn apply_focus(&self, focused: u8, target: u8, element: u32) -> bool {
        if focused > 1 {
            return false;
        }
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return false;
        };
        if !state.created {
            return false;
        }
        match target {
            EDITOR_OPS_TARGET_EDITOR => {
                if element != 0 {
                    return false;
                }
                state.record(format!("f:{focused}"));
                if focused == 1
                    && let Some(hwnd) = state.hwnd
                {
                    win32::set_focus(hwnd);
                }
            }
            EDITOR_OPS_TARGET_ELEMENT => {
                state.record(format!("fe:{element}:{focused}"));
            }
            _ => return false,
        }
        true
    }

    /// `popup` — `Dialog` opens a real `WS_POPUP` owned by the
    /// editor child's root ancestor (`GetAncestor(GA_ROOT)` — the
    /// top-level inventory oracle's anchor), `ChildWindow` opens a
    /// `WS_CHILD` under the editor child (the `EnumChildWindows`
    /// observation point); both carry the generation title
    /// `rvf-test-editor-popup-{seq}`. `po:{seq}`/`pc:{seq}` is
    /// recorded whether or not the HWND exists — logical and HWND
    /// faces stay separate so the non-Windows shim keeps token
    /// honesty. The verb popup surface is a single slot: close
    /// converges it regardless of `kind`. Defined rejections:
    /// strict-flag violation, a re-entrant borrow, no created GUI,
    /// an unassigned `kind`, or a repeated edge (open on an open
    /// slot, close on a closed one).
    fn popup(&self, kind: u8, open: u8) -> bool {
        if open > 1 {
            return false;
        }
        if kind != EDITOR_OPS_POPUP_CHILD && kind != EDITOR_OPS_POPUP_DIALOG {
            return false;
        }
        let open = open == 1;
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return false;
        };
        if !state.created {
            return false;
        }
        if open == state.verb_popup_open {
            return false;
        }
        if open {
            state.verb_popup_seq = state.verb_popup_seq.wrapping_add(1);
            let seq = state.verb_popup_seq;
            let title = format!("rvf-test-editor-popup-{seq}");
            state.verb_popup = if kind == EDITOR_OPS_POPUP_CHILD {
                state
                    .hwnd
                    .and_then(|hwnd| win32::child(hwnd, &title, VERB_POPUP_SIZE))
            } else {
                let owner = state.hwnd.and_then(win32::root_ancestor);
                win32::popup(owner, &title, VERB_POPUP_SIZE)
            };
            state.verb_popup_open = true;
            state.record(format!("po:{seq}"));
        } else {
            if let Some(hwnd) = state.verb_popup.take() {
                win32::destroy(hwnd);
            }
            state.verb_popup_open = false;
            let seq = state.verb_popup_seq;
            state.record(format!("pc:{seq}"));
        }
        true
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

    use super::GuiSize;

    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows_sys::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleHandleExW,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GA_ROOT,
        GetAncestor, GetClassInfoW, GetClientRect, IsWindow, RegisterClassW, SW_HIDE, SW_SHOW,
        SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, SetWindowTextW, ShowWindow, WNDCLASSW, WS_CHILD,
        WS_CLIPSIBLINGS, WS_POPUP, WS_VISIBLE,
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
    /// before any Win32 mutation. `title` becomes the window text —
    /// the attach-generation tag + verb event log an observer reads
    /// back through `GetWindowTextW` (the VST3 `attach` precedent).
    ///
    /// # Returns
    /// `Some` child HWND handle (`usize`) on success, `None` on any
    /// defined rejection — the caller maps `None` to a
    /// `PluginError`.
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
        // owned by this gui state until `destroy`.
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

    /// Creates the plugin-owned popup window — a `WS_POPUP` HWND
    /// owned by `owner` (the `set_transient` parent, so the popup
    /// stays above it and dies with it; `None` → unowned), sized
    /// `width`×`height`, titled `title` — the observable tag an
    /// observer reads through `GetWindowTextW`. Returns `None` on any
    /// defined failure — the caller maps it to a `PluginError`.
    pub(super) fn popup(owner: Option<usize>, title: &str, size: GuiSize) -> Option<usize> {
        if !ensure_registered() {
            return None;
        }
        let class = class_name();
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: the class and title outlive the synchronous call;
        // `owner` is a live window or null (unowned popup). The
        // returned HWND is owned by this gui state until `destroy`.
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title_w.as_ptr(),
                WS_POPUP | WS_CLIPSIBLINGS,
                0,
                0,
                i32::try_from(size.width).unwrap_or(200),
                i32::try_from(size.height).unwrap_or(100),
                owner.map_or(ptr::null_mut(), |hwnd| hwnd as HWND),
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

    /// Whether `hwnd` currently names a live window — the
    /// `set_transient` validation gate and the test-side existence
    /// check.
    pub(super) fn is_window(hwnd: usize) -> bool {
        // SAFETY: `IsWindow` is a pure handle-validity query — safe
        // on any value.
        unsafe { IsWindow(hwnd as HWND) != 0 }
    }

    /// Stamps the popup HWND's window text — the observable effect
    /// of `suggest_title`. Best-effort: a dead handle is a harmless
    /// `SetWindowTextW` error.
    pub(super) fn set_title(hwnd: usize, title: &str) {
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `hwnd` is a window this module created; on a dead
        // handle the call is a defined no-op error.
        unsafe {
            SetWindowTextW(hwnd as HWND, title_w.as_ptr());
        }
    }

    /// Moves keyboard focus to `hwnd` — the real `SetFocus` leg of
    /// the `apply_focus` verb, giving host-side checks a
    /// `GetFocus() == child` oracle. Best-effort: a dead handle or
    /// an OS refusal (e.g. a hidden ancestor chain) is a harmless
    /// NULL return, not a delivery failure.
    pub(super) fn set_focus(hwnd: usize) {
        // SAFETY: `hwnd` is a window this module created (or a dead
        // handle, where `SetFocus` is a defined no-op error) — and
        // the caller runs on the parent HWND's pump thread where the
        // call is legal.
        unsafe {
            SetFocus(hwnd as HWND);
        }
    }

    /// The root ancestor of `hwnd` — `GetAncestor(GA_ROOT)`: the
    /// top-level window a `Dialog`-kind popup is owned by (the same
    /// anchor the VST3 popup resolves). `None` on a dead or
    /// non-window handle.
    pub(super) fn root_ancestor(hwnd: usize) -> Option<usize> {
        // SAFETY: `IsWindow`/`GetAncestor` are pure handle queries —
        // safe on any value.
        let ancestor = unsafe {
            if IsWindow(hwnd as HWND) == 0 {
                return None;
            }
            GetAncestor(hwnd as HWND, GA_ROOT)
        };
        if ancestor.is_null() {
            None
        } else {
            Some(ancestor as usize)
        }
    }

    /// Creates the `ChildWindow`-kind popup — a `WS_CHILD |
    /// WS_VISIBLE | WS_CLIPSIBLINGS` HWND nested under the editor
    /// child `parent`, titled `rvf-test-editor-popup-{seq}` (the
    /// `EnumChildWindows` observation point). Returns `None` on any
    /// defined failure — the caller still logs the verb.
    pub(super) fn child(parent: usize, title: &str, size: GuiSize) -> Option<usize> {
        // SAFETY: `IsWindow` is a pure handle-validity query — safe
        // on any value.
        if unsafe { IsWindow(parent as HWND) } == 0 {
            return None;
        }
        if !ensure_registered() {
            return None;
        }
        let class = class_name();
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `parent` passed the `IsWindow` gate; class and
        // title outlive the synchronous call; the returned HWND is
        // owned by this gui state until `destroy`.
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title_w.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                i32::try_from(size.width).unwrap_or(200),
                i32::try_from(size.height).unwrap_or(100),
                parent as HWND,
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
}

/// Non-Windows shim — no HWND surface exists off Windows, so
/// `attach`/`popup` always answer `None` (the caller's defined
/// rejection) and the rest are unreachable but defined.
#[cfg(not(target_os = "windows"))]
mod win32 {
    use std::ffi::c_void;

    use super::GuiSize;

    pub(super) fn attach(_parent: *mut c_void, _title: &str) -> Option<usize> {
        None
    }

    pub(super) fn set_visible(_hwnd: usize, _visible: bool) {}

    pub(super) fn resize(_hwnd: usize, _width: i32, _height: i32) {}

    pub(super) fn destroy(_hwnd: usize) {}

    pub(super) fn popup(_owner: Option<usize>, _title: &str, _size: GuiSize) -> Option<usize> {
        None
    }

    pub(super) fn is_window(_hwnd: usize) -> bool {
        false
    }

    pub(super) fn set_title(_hwnd: usize, _title: &str) {}

    pub(super) fn set_focus(_hwnd: usize) {}

    pub(super) fn root_ancestor(_hwnd: usize) -> Option<usize> {
        None
    }

    pub(super) fn child(_parent: usize, _title: &str, _size: GuiSize) -> Option<usize> {
        None
    }
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
        let mut processor = TestEditorProcessor::new();
        let mut context = ProcessContext::new();
        processor
            .prepare(&config, &mut context)
            .map_err(|_| fail("processor preparation failed"))?;
        shared
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(|_| PluginError::Message("activate outside Initialized state"))?;
        // Activation resets the armed value to the default (the
        // fixture's deterministic per-activation law).
        shared.fault_mirror.store(FAULT_PARAM.default_value);
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

        // Mirror sync-in, then sample-accurate event application, then
        // sync-out — keeps `params.value` honest on the main thread.
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
        let event_list =
            block::translate_events(events.input, &mut self.scratch, frames_count, |unknown| {
                block::translate_param_event(unknown, &FAULT_PARAM)
            })?;
        let inputs = [input_view];
        let mut outputs = [output_view];
        let result =
            self.processor
                .process(&inputs, &mut outputs, event_list, None, &mut self.context);
        self.shared.fault_mirror.store(self.processor.armed_plain());
        // The CLAP-only fault legs: `bad_time` emits a `ParamValue`
        // stamped beyond any legal block bound, `invalid` one
        // carrying NaN — real ABI-level malformed events a spec host
        // observes on its output list. The ruviform host drains them
        // into canonical reports (Issue #842); the push stays
        // best-effort and its failure is ignored.
        match decode_fault(self.processor.armed_plain()).0 {
            FaultMode::BadTime => {
                let _ = events.output.try_push(bad_time_output_event());
            }
            FaultMode::Invalid => {
                let _ = events.output.try_push(invalid_output_event());
            }
            _ => {}
        }
        block::map_process_status(result)
    }

    fn deactivate(self, _main_thread: &TestEditorMainThread<'a>) {
        let _ = self.shared.lifecycle.transition(LifecycleEvent::Deactivate);
    }

    fn reset(&mut self) {
        // CLAP `reset` clears processing state but not parameter
        // values — the armed value *is* the parameter value, so reset
        // resyncs from the authoritative mirror.
        self.processor
            .set_armed_plain(self.shared.fault_mirror.load());
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

    use clack_common::events::Pckn;
    use clack_common::events::event_types::ParamValueEvent;
    use clack_common::events::io::EventBuffer;
    use clack_common::events::spaces::CoreEventSpace;
    use clack_host::process::audio_buffers::{
        AudioPortBuffer, AudioPortBufferType, AudioPorts, InputAudioBuffers, InputChannel,
        OutputAudioBuffers,
    };

    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, ParamEvent, ParamId, ParamValue, ProcessStatus};

    use crate::fault::RT_ALLOC_FILL;

    /// A fresh main-thread object bound to `shared` — the params
    /// surface reads the shared fault mirror; the gui state is
    /// self-contained.
    fn main_thread(shared: &TestEditorShared) -> TestEditorMainThread<'_> {
        TestEditorMainThread {
            gui: RefCell::new(GuiState::default()),
            shared,
        }
    }

    /// `is_api_supported` answers `true` only for `win32` on Windows
    /// builds — embedded *and* floating (the popup surface); every
    /// other API type and non-Windows builds are honest `false`.
    #[test]
    fn api_support_is_win32_embedded_on_windows_only() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        assert_eq!(
            main.is_api_supported(EMBEDDED_WIN32),
            cfg!(windows),
            "win32+embedded is supported"
        );
        let floating = GuiConfiguration {
            api_type: GuiApiType::WIN32,
            is_floating: true,
        };
        assert_eq!(
            main.is_api_supported(floating),
            cfg!(windows),
            "win32+floating is the popup surface"
        );
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
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
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
        // `set_transient` before `create` is a protocol violation —
        // the floating-mode verb still declines honestly here.
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
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        // A non-win32 configuration is a defined rejection.
        let cocoa = GuiConfiguration {
            api_type: GuiApiType::COCOA,
            is_floating: false,
        };
        assert!(main.create(cocoa).is_err());
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

    /// Floating mode is the fixture's popup surface: `set_parent`
    /// rejects (a floating surface has no embed), `set_transient`
    /// accepts only a live Win32 window, `suggest_title` names the
    /// popup, and `show` creates a real plugin-owned `WS_POPUP`
    /// HWND that `hide`/`destroy` converge. Off Windows every verb
    /// stays a defined rejection.
    #[test]
    fn floating_popup_surface_is_honest() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        let floating = GuiConfiguration {
            api_type: GuiApiType::WIN32,
            is_floating: true,
        };
        if !cfg!(windows) {
            assert!(main.create(floating).is_err());
            return;
        }
        assert!(main.create(floating).is_ok());
        // A floating surface cannot be embedded under a parent.
        let parent = Window::from_generic_ptr(GuiApiType::WIN32, ptr::null_mut());
        assert!(main.set_parent(parent).is_err());
        // `set_transient` gates on a *live* win32 window — null and
        // dead payloads are the defined rejection.
        let dead = Window::from_generic_ptr(GuiApiType::WIN32, ptr::null_mut());
        assert!(main.set_transient(dead).is_err());
        #[cfg(target_os = "windows")]
        {
            // The host's window the popup stays above — a real HWND
            // created with the same helper the popup itself uses.
            let anchor = win32::popup(None, "transient-anchor", DEFAULT_GUI_SIZE)
                .expect("transient anchor window");
            let transient =
                Window::from_generic_ptr(GuiApiType::WIN32, anchor as *mut core::ffi::c_void);
            assert!(main.set_transient(transient).is_ok());
            main.suggest_title("rvf-popup-suggested");
            // First `show` materializes the plugin-owned popup.
            assert!(main.show().is_ok());
            assert!(
                win32::is_window(main.gui.borrow().hwnd.expect("popup hwnd")),
                "floating show leaves a live popup HWND"
            );
            assert!(main.hide().is_ok());
            assert!(main.show().is_ok(), "re-showing a live popup");
            main.destroy();
            assert!(main.gui.borrow().hwnd.is_none(), "destroy frees the popup");
            win32::destroy(anchor);
        }
    }

    /// `set_parent` rejects honestly on every platform: a non-Win32
    /// `Window`, a null HWND payload, and a payload that is not a
    /// live window all fail before any child HWND exists — and
    /// `set_parent` before `create` is a protocol violation.
    #[test]
    fn set_parent_rejects_invalid_parents_honestly() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
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
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        assert!(main.show().is_err());
        assert!(main.hide().is_err());
    }

    /// The DSP law at the neutral-contract layer: one block copies
    /// `in` to `out` bitwise and reports `Continue`; empty buses
    /// report `Sleep`.
    #[test]
    fn processor_is_bitwise_pass_through() {
        let mut processor = TestEditorProcessor::new();
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

    // -- Fault arm (the `test_fault` law delegated through the editor) ----

    fn fault_event(normalized: f64) -> Event {
        Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: FAULT_PARAM.id,
                value: ParamValue::new(normalized).unwrap(),
            }),
        )
    }

    /// Runs one block through the neutral contract — only `none`/
    /// `error`/`rt_alloc`/`bad_time`/`invalid` modes are exercised
    /// here: `abort`/`exit`/`hang` would kill or wedge the test
    /// binary itself.
    fn run_block(
        processor: &mut TestEditorProcessor,
        input: &[Vec<f32>],
        output: &mut [Vec<f32>],
        events: &[Event],
    ) -> ProcessResult {
        let frames = u32::try_from(input.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(events, frames).unwrap();
        let mut context = ProcessContext::new();
        processor.process(&inputs, &mut outputs, list, None, &mut context)
    }

    /// The `error` mode returns `ProcessStatus::Error` — a defined
    /// rejection the host observes as a failed result, not a crash.
    #[test]
    fn error_mode_returns_process_error() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 4.0 → normalized 0.5 → `error`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.5)]);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    /// `rt_alloc` allocates inside `process()` and writes the heap
    /// byte (`200.0` = `f32::from(0xC8)` — `RT_ALLOC_FILL`) into
    /// `output[0][0]` — the sample proves the allocation round-trip
    /// ran. The rest of the block stays pass-through.
    #[test]
    fn alloc_fault_marks_armed_block() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 5.0 → normalized 0.625 → `rt_alloc`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.625)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0][0], f32::from(RT_ALLOC_FILL));
        assert_eq!(output[0][1..], input[0][1..]);
        assert_eq!(output[1], input[1]);
    }

    /// `bad_time` returns `Continue` with `TAIL_INFINITE` — the
    /// zero-tail fixture's timing lie surfaces on the neutral
    /// `tail_frames` a host reads back.
    #[test]
    fn bad_time_mode_claims_infinite_tail() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 6.0 → normalized 0.75 → `bad_time`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.75)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(result.tail_frames, ProcessResult::TAIL_INFINITE);
    }

    /// `invalid` fills every output sample with NaN — the invalid
    /// audio state the host oracle detects on the delivered samples.
    #[test]
    fn invalid_mode_fills_nan_outputs() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert!(output[0].iter().all(|s| s.is_nan()));
        assert!(output[1].iter().all(|s| s.is_nan()));
    }

    /// The armed modes persist until disarmed: after `invalid`, a
    /// `none` event restores pure pass-through — the fault mode's
    /// recovery half.
    #[test]
    fn disarm_restores_pass_through() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        // plain 0.0 → normalized 0.0 → `none`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.0)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    /// RT proof: the unarmed/`none` path never allocates — the armed
    /// `rt_alloc` path is the deliberate violation; the inert law is
    /// untouched. Views and the event list are built outside the
    /// `assert_no_alloc` window so only `process()` itself is
    /// measured (the `rt_` name selects this for `mise run test-rt`).
    #[test]
    fn rt_unarmed_block_is_alloc_free() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let events = [fault_event(0.0)];
        let list = EventList::new(&events, 8).unwrap();
        let mut context = ProcessContext::new();
        assert_no_alloc::assert_no_alloc(|| {
            processor.process(&inputs, &mut outputs, list, None, &mut context)
        });
    }

    /// Foreign parameter ids never arm the fault.
    #[test]
    fn foreign_param_events_do_not_arm() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let foreign = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(9),
                value: ParamValue::new(1.0).unwrap(),
            }),
        );
        let result = run_block(&mut processor, &input, &mut output, &[foreign]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    // -- ABI-level output events (the CLAP-only legs) --------------------

    /// The stereo port fixture a host builds for one `clap_process`
    /// call — `clack_host`'s safe buffer builders over owned `Vec`s,
    /// the same shape as `fault.rs`'s `StereoFixture`.
    struct StereoFixture {
        inputs: [Vec<f32>; block::MAX_CHANNELS],
        outputs: [Vec<f32>; block::MAX_CHANNELS],
        input_ports: AudioPorts,
        output_ports: AudioPorts,
    }

    impl StereoFixture {
        fn new(frames: usize) -> Self {
            Self {
                inputs: [vec![0.5; frames], vec![-0.25; frames]],
                outputs: [vec![0.0; frames], vec![0.0; frames]],
                input_ports: AudioPorts::with_capacity(block::MAX_CHANNELS, 1),
                output_ports: AudioPorts::with_capacity(block::MAX_CHANNELS, 1),
            }
        }

        fn guards(&mut self) -> (InputAudioBuffers<'_>, OutputAudioBuffers<'_>) {
            let Self {
                inputs,
                outputs,
                input_ports,
                output_ports,
            } = self;
            let input_buffers = input_ports.with_input_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_input_only(
                    inputs.iter_mut().map(InputChannel::variable),
                ),
            }]);
            let output_buffers = output_ports.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only(
                    outputs.iter_mut().map(Vec::as_mut_slice),
                ),
            }]);
            (input_buffers, output_buffers)
        }
    }

    /// An initialized shared plus an activated audio processor —
    /// the legal path every ABI-level test starts from.
    fn activated(shared: &TestEditorShared) -> TestEditorAudioProcessor<'_> {
        let config = PluginAudioConfiguration {
            sample_rate: 48_000.0,
            min_frames_count: 1,
            max_frames_count: 64,
        };
        TestEditorAudioProcessor::activate_inner(shared, &config).unwrap()
    }

    /// One `process` call through the ABI-level
    /// [`TestEditorAudioProcessor`] — the shape a host builds for
    /// `clap_process` (`Process`/`Events` plumbing plus the event
    /// buffers).
    fn process_abi(
        ap: &mut TestEditorAudioProcessor<'_>,
        fixture: &mut StereoFixture,
        input_buffer: &EventBuffer,
        output_buffer: &mut EventBuffer,
    ) -> ClapProcessStatus {
        let (input_buffers, mut output_buffers) = fixture.guards();
        let audio = output_buffers.as_plugin_audio_with_inputs(&input_buffers);
        let input_events = input_buffer.as_input();
        let mut output_events = output_buffer.as_output();
        ap.process(
            Process {
                transport: None,
                steady_time: None,
            },
            audio,
            Events {
                input: &input_events,
                output: &mut output_events,
            },
        )
        .unwrap()
    }

    /// The `ParamValue` events the plugin emitted into the output
    /// buffer — `(time, value)` pairs.
    fn emitted_param_values(output_buffer: &EventBuffer) -> Vec<(u32, f64)> {
        output_buffer
            .as_input()
            .iter()
            .filter_map(|unknown| match unknown.as_core_event() {
                Some(CoreEventSpace::ParamValue(event)) => {
                    Some((unknown.header().time(), event.value()))
                }
                _ => None,
            })
            .collect()
    }

    /// `bad_time` through the real ABI: a plain-`6.0` `ParamValue`
    /// input event arms the mode, `process` reports
    /// `CLAP_PROCESS_TAIL`, and the output event list carries a
    /// `ParamValue` stamped `u32::MAX` — the ABI-level bad timestamp
    /// a spec host observes.
    #[test]
    fn bad_time_mode_emits_tail_and_bad_time_event() {
        let shared = TestEditorShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            6.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Tail);
        let emitted = emitted_param_values(&output_buffer);
        assert!(
            emitted.iter().any(|&(time, _)| time == u32::MAX),
            "bad_time must emit a u32::MAX-stamped event, got {emitted:?}"
        );
    }

    /// `invalid` through the real ABI: the output audio is all NaN
    /// and the output event list carries a `ParamValue` event whose
    /// value is non-finite — the ABI-level malformed payload.
    #[test]
    fn invalid_mode_emits_nan_and_invalid_event() {
        let shared = TestEditorShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            7.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Continue);
        assert!(fixture.outputs[0].iter().all(|s| s.is_nan()));
        assert!(fixture.outputs[1].iter().all(|s| s.is_nan()));
        let emitted = emitted_param_values(&output_buffer);
        assert!(
            emitted.iter().any(|&(_, value)| value.is_nan()),
            "invalid must emit a NaN-valued event, got {emitted:?}"
        );
    }

    /// `rt_alloc` through the real ABI: the heap marker lands in the
    /// delivered output buffer — `outputs[0][0] == 200.0` while the
    /// rest of the block stays pass-through.
    #[test]
    fn alloc_fault_marks_armed_abi_output() {
        let shared = TestEditorShared::initialize().unwrap();
        let mut ap = activated(&shared);
        ap.start_processing().unwrap();

        let mut fixture = StereoFixture::new(64);
        let mut input_buffer = EventBuffer::new();
        input_buffer.push(&ParamValueEvent::new(
            0,
            ClapId::new(FAULT_PARAM.id.0),
            Pckn::match_all(),
            5.0,
        ));
        let mut output_buffer = EventBuffer::new();
        let status = process_abi(&mut ap, &mut fixture, &input_buffer, &mut output_buffer);
        assert_eq!(status, ClapProcessStatus::Continue);
        assert_eq!(fixture.outputs[0][0], f32::from(RT_ALLOC_FILL));
        assert_eq!(fixture.outputs[0][1..], fixture.inputs[0][1..]);
        assert_eq!(fixture.outputs[1], fixture.inputs[1]);
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

        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
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

    // -- `ruviform.editor-ops` (Issue #630 private seam) ------------------

    /// `ruviform.editor-ops` defined rejections — every verb refuses
    /// before `create` (no GUI surface), and the strict flag /
    /// discriminant grammar is enforced: unassigned
    /// direction/target/kind bytes, `focused`/`open` outside
    /// `{0,1}`, `element != 0` on a non-element target, a repeated
    /// popup edge, and key fields wider than the 16-bit slots.
    #[test]
    fn editor_ops_verbs_reject_undefined_inputs() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        // No created GUI — every verb is the defined `false`
        // rejection on every OS.
        assert!(!main.deliver_key(EDITOR_OPS_KEY_DOWN, 0x61, 0x41, 0));
        assert!(!main.apply_focus(1, EDITOR_OPS_TARGET_EDITOR, 0));
        assert!(!main.popup(EDITOR_OPS_POPUP_DIALOG, 1));
        if !cfg!(windows) {
            // `create` declines off Windows — the verbs stay refused.
            return;
        }
        assert!(main.create(EMBEDDED_WIN32).is_ok());
        // Unassigned discriminant bytes.
        assert!(!main.deliver_key(9, 0x61, 0x41, 0));
        assert!(!main.apply_focus(1, 9, 0));
        assert!(!main.popup(9, 1));
        // Strict {0,1} flags.
        assert!(!main.apply_focus(2, EDITOR_OPS_TARGET_EDITOR, 0));
        assert!(!main.popup(EDITOR_OPS_POPUP_DIALOG, 2));
        // `element` is `EditorElement`-only payload.
        assert!(!main.apply_focus(1, EDITOR_OPS_TARGET_EDITOR, 5));
        // Out-of-range key fields (> u16 contract slots).
        assert!(!main.deliver_key(EDITOR_OPS_KEY_DOWN, 0x1_0000, 0x41, 0));
        assert!(!main.deliver_key(EDITOR_OPS_KEY_DOWN, 0x61, 0x1_0000, 0));
        assert!(!main.deliver_key(EDITOR_OPS_KEY_DOWN, 0x61, 0x41, 0x1_0000));
        // A repeated popup edge is a defined rejection.
        assert!(main.popup(EDITOR_OPS_POPUP_DIALOG, 1));
        assert!(!main.popup(EDITOR_OPS_POPUP_DIALOG, 1));
        main.destroy();
    }

    /// The `ruviform.editor-ops` verbs record the shared token
    /// grammar into the event log — `f:`/`fe:`/`kd:`/`ku:`/`po:`/
    /// `pc:` — and `window_title` stamps them behind the
    /// attach-generation tag, the `GetWindowTextW` channel an
    /// observer reads (synthetic verb delivery).
    #[test]
    fn editor_ops_verbs_record_event_log_tokens() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        if !cfg!(windows) {
            // `create` declines off Windows — the `!created`
            // rejection above is the coverage there.
            return;
        }
        assert!(main.create(EMBEDDED_WIN32).is_ok());
        assert!(main.apply_focus(1, EDITOR_OPS_TARGET_EDITOR, 0));
        assert!(main.apply_focus(1, EDITOR_OPS_TARGET_ELEMENT, 7));
        assert!(main.deliver_key(EDITOR_OPS_KEY_DOWN, 0x61, 0x41, 0x1));
        assert!(main.deliver_key(EDITOR_OPS_KEY_UP, 0x61, 0x41, 0));
        assert!(main.apply_focus(0, EDITOR_OPS_TARGET_EDITOR, 0));
        assert!(main.popup(EDITOR_OPS_POPUP_DIALOG, 1));
        assert!(main.popup(EDITOR_OPS_POPUP_DIALOG, 0));
        // `ChildWindow` under an unattached editor: no HWND exists,
        // but the logical open + tokens still land (the HWND-
        // independent oracle).
        assert!(main.popup(EDITOR_OPS_POPUP_CHILD, 1));
        assert!(main.popup(EDITOR_OPS_POPUP_CHILD, 0));
        {
            let state = main.gui.borrow();
            assert_eq!(
                state.events.as_slice(),
                [
                    "f:1",
                    "fe:7:1",
                    "kd:0061:0041:0001",
                    "ku:0061:0041:0000",
                    "f:0",
                    "po:1",
                    "pc:1",
                    "po:2",
                    "pc:2",
                ],
                "verb deliveries record the shared token grammar"
            );
            assert_eq!(
                state.window_title(),
                "rvf-test-editor-attach-0|f:1,fe:7:1,kd:0061:0041:0001,\
                 ku:0061:0041:0000,f:0,po:1,pc:1,po:2,pc:2"
            );
        }
        main.destroy();
    }

    /// `destroy` frees the verb popup HWND and resets the logical
    /// open state — the same teardown the child HWND gets.
    #[test]
    fn destroy_tears_down_the_verb_popup() {
        let shared = TestEditorShared::initialize().unwrap();
        let main = main_thread(&shared);
        if !cfg!(windows) {
            return;
        }
        assert!(main.create(EMBEDDED_WIN32).is_ok());
        assert!(main.popup(EDITOR_OPS_POPUP_DIALOG, 1));
        let popup = {
            let state = main.gui.borrow();
            state.verb_popup.expect("dialog popup hwnd on windows")
        };
        assert!(win32::is_window(popup), "open leaves a live popup HWND");
        main.destroy();
        let state = main.gui.borrow();
        assert!(state.verb_popup.is_none() && !state.verb_popup_open);
        assert!(!win32::is_window(popup), "destroy frees the popup HWND");
    }
}
