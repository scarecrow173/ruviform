//! Native Dock layout scenario (Issue #388): the plugin editor slot is a
//! `NativeSlot` component inside a GPUI Panel/Tab layout. The renderer
//! projects the laid-out bounds, the ancestor clip and inactive-tab
//! suppression onto the driver's canonical slot; this scenario drives
//! the tab switch through [`GpuiDriver::update_view`] and the resize
//! through the window driver, and records every check as a
//! `[poc] check=…` evidence line.
//!
//! Selected with `RVF_POC_SCENARIO=dock`. The scenario reports checks
//! only; the Dock PASS decision is taken from the evidence report, not
//! from this process exit alone.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use ruviform_plugin_core::ParentLease;
use ruviform_plugin_host::{
    AttachMode, EditorOpError, EditorSessionState, InProcessEditorOps, InProcessHost,
    ParentSlotLease, PluginEditorSessionId, PluginEditorSessions, PluginHandle,
};
use ruviform_ui::view::{
    A11ySpec, AttachEpoch, ComponentKey, ComponentSpec, ComponentVisualState, NativeSlotSpec,
    PanelSpec, TabSpec, TabsSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{
    CloseDecision, DriverEvent, LogicalRect, LogicalSize, SlotCapabilities, SlotError, SlotEvent,
    SlotGeometry, SlotLease, SlotSpec, SlotVisibility, Support, SurfaceSlotId, ViewInstanceId,
    WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::{GpuiDriver, SharedSurface, ViewMount};

use super::{
    ProbeShared, SIZE, ev, first_child_of, focus_hwnd, instantiate, is_visible, is_window,
    parent_of, window_rect,
};

const SLOT_KEY: &str = "dock.editor";
const TAB_EDITOR: usize = 0;
const TAB_OTHER: usize = 1;
const SETTLE_TICKS: u32 = 20;
const PROJECTION_TIMEOUT: u32 = 600;
const WINDOW_SIZE: LogicalSize = LogicalSize {
    width: 1000,
    height: 720,
};
const GROW_SIZE: LogicalSize = LogicalSize {
    width: 1280,
    height: 900,
};
const SHRINK_SIZE: LogicalSize = LogicalSize {
    width: 520,
    height: 560,
};

/// Ordered scenario stages. Each stage waits `SETTLE_TICKS` ticks (or
/// the projection timeout) before its observation runs.
#[derive(Copy, Clone, Debug)]
enum Stage {
    WaitReady,
    AwaitProjection,
    Attached,
    TabAway,
    TabBack,
    ResizeGrow,
    ResizeShrink,
    Done,
}

/// The Panel → Tabs layout: the editor tab holds the `NativeSlot`, the
/// other tab holds no native surface.
fn dock_view(active: usize) -> ViewSpec {
    let editor = ComponentSpec::NativeSlot(NativeSlotSpec {
        key: ComponentKey(SLOT_KEY.to_string()),
        min_size: Some([SIZE.width, SIZE.height]),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    });
    let other = ComponentSpec::Panel(PanelSpec {
        key: ComponentKey("dock.other".to_string()),
        title: None,
        children: vec![],
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    });
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("dock.root".to_string()),
            title: None,
            children: vec![ComponentSpec::Tabs(TabsSpec {
                key: ComponentKey("dock.tabs".to_string()),
                active,
                tabs: vec![
                    TabSpec {
                        key: ComponentKey("tab.editor".to_string()),
                        title: "Editor".to_string(),
                        child: Box::new(editor),
                    },
                    TabSpec {
                        key: ComponentKey("tab.other".to_string()),
                        title: "Other".to_string(),
                        child: Box::new(other),
                    },
                ],
                selection: None,
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            })],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// Redeemer composition against the driver's canonical surface — the
/// same ledger the renderer projects geometry onto.
fn dock_ops(
    host: &mut InProcessHost,
    surface: SharedSurface,
    instance: PluginHandle,
) -> InProcessEditorOps<'_> {
    InProcessEditorOps::with_redeemer(
        host,
        instance,
        Box::new(move |lease: &ParentSlotLease| {
            surface
                .redeem(lease.raw())
                .map(|token| ParentLease::new(SlotLease::new(lease.raw()).seq(), token))
                .map_err(|error| match error {
                    SlotError::Unsupported => EditorOpError::Unsupported,
                    _ => EditorOpError::LeaseRejected,
                })
        }),
    )
}

/// `(x, y, w, h)` in logical px of a slot rect.
type LogicalRectTuple = (f64, f64, f64, f64);

fn rect_tuple(rect: LogicalRect) -> LogicalRectTuple {
    (rect.x, rect.y, rect.width, rect.height)
}

/// The foreground window handle, for evidence screenshots.
#[expect(unsafe_code)]
fn foreground_hwnd() -> usize {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: query only.
    unsafe { GetForegroundWindow() as usize }
}

/// Captures the screen region of `hwnd` into `<evidence>/<label>.png`
/// through PowerShell `CopyFromScreen`. Skipped without
/// `RVF_POC_EVIDENCE_DIR`.
fn screenshot(evidence_dir: Option<&Path>, label: &str, hwnd: usize) {
    let Some(dir) = evidence_dir else {
        return;
    };
    let Some((x, y, w, h)) = window_rect(hwnd) else {
        ev(&format!("screenshot={label} result=skip reason=no_window"));
        return;
    };
    let path = dir.join(format!("{label}.png"));
    let quoted = path.display().to_string().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Drawing; \
         $b = New-Object System.Drawing.Bitmap {w},{h}; \
         $g = [System.Drawing.Graphics]::FromImage($b); \
         $g.CopyFromScreen({x},{y},0,0,$b.Size); \
         $b.Save('{quoted}', [System.Drawing.Imaging.ImageFormat]::Png); \
         $g.Dispose(); $b.Dispose()"
    );
    let ok = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .is_ok_and(|s| s.success());
    ev(&format!(
        "screenshot={label} result={} path={} window_origin=({x},{y}) window_size={w}x{h} foreground_is_window={}",
        if ok { "ok" } else { "fail" },
        path.display(),
        foreground_hwnd() == hwnd
    ));
}

struct Dock {
    driver: GpuiDriver,
    surface: SharedSurface,
    window: WindowId,
    host: InProcessHost,
    sessions: PluginEditorSessions,
    session: PluginEditorSessionId,
    instance: PluginHandle,
    probe: Rc<ProbeShared>,
    vst3_mode: bool,
    evidence_dir: Option<PathBuf>,
    failures: Rc<Cell<u32>>,
    slot: Option<SurfaceSlotId>,
    lease: Option<SlotLease>,
    /// Slot container HWND (the parent the editor attaches under).
    container: usize,
    child: usize,
    rect_last: Option<LogicalRectTuple>,
    stage: Stage,
    /// Ticks spent in the current stage.
    ticks: u32,
}

impl Dock {
    /// Records one check; a failed check is counted for the exit code.
    fn check(&self, id: &str, ok: bool, detail: &str) {
        if !ok {
            self.failures.set(self.failures.get() + 1);
        }
        ev(&format!(
            "check={id} result={} {detail}",
            if ok { "pass" } else { "fail" }
        ));
    }

    fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        self.ticks = 0;
    }

    fn slot_spec(&self) -> Option<SlotSpec> {
        self.surface.spec(self.slot?).ok().flatten()
    }

    fn slot_rect(&self) -> Option<LogicalRectTuple> {
        self.slot_spec().map(|s| rect_tuple(s.geometry.rect))
    }

    fn slot_visibility(&self) -> Option<SlotVisibility> {
        self.slot_spec().map(|s| s.visibility)
    }

    fn editor_visible(&self) -> bool {
        is_visible(self.child)
    }

    /// Ready → declare the slot on the canonical surface and bind the
    /// layout key to it. The renderer projects geometry on paint.
    fn on_ready(&mut self, drv: &mut dyn WindowDriver) {
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
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                scale_factor: scale,
            },
            visibility: SlotVisibility::Hidden,
            capabilities: caps,
        };
        let slot = match self.surface.declare_slot(spec) {
            Ok(slot) => slot,
            Err(e) => {
                self.check("declare_slot", false, &format!("error={e}"));
                self.enter(Stage::Done);
                return;
            }
        };
        self.slot = Some(slot);
        // Screenshots are only trusted while this window is foreground
        // (`foreground_is_window=true`), so activate it up front.
        let activated = drv.focus(self.window, true);
        ev(&format!("activate_window={}", activated.is_ok()));
        let bound =
            self.driver
                .bind_native_slot(self.window, ComponentKey(SLOT_KEY.to_string()), slot);
        self.check(
            "bind_native_slot",
            bound.is_ok(),
            &format!("slot_index={} error={bound:?}", slot.index()),
        );
        self.enter(if bound.is_ok() {
            Stage::AwaitProjection
        } else {
            Stage::Done
        });
    }

    /// Waits for the renderer to project a non-empty, visible geometry
    /// for the bound slot — the dock layout is the only source of it.
    fn await_projection(&mut self) {
        let Some(rect) = self.slot_rect() else {
            self.check("projection", false, "slot_spec=unavailable");
            self.enter(Stage::Done);
            return;
        };
        let visible = self.slot_visibility() == Some(SlotVisibility::Visible);
        if rect.2 > 0.0 && rect.3 > 0.0 && visible {
            ev(&format!("projected_rect={rect:?} visibility=Visible"));
            self.attach();
        } else if self.ticks >= PROJECTION_TIMEOUT {
            self.check(
                "projection",
                false,
                &format!("rect={rect:?} visibility={:?}", self.slot_visibility()),
            );
            self.enter(Stage::Done);
        }
    }

    /// Leases the projected slot and opens the editor session against
    /// the lease — the same path as the floating probe, but the parent
    /// container is owned by the Dock slot.
    fn attach(&mut self) {
        let slot = self.slot.expect("slot declared");
        let lease = match self.surface.lease_parent(slot) {
            Ok(lease) => lease,
            Err(e) => {
                self.check("lease_parent", false, &format!("error={e}"));
                self.enter(Stage::Done);
                return;
            }
        };
        self.lease = Some(lease);
        self.container = usize::try_from(
            self.surface
                .container_token(slot)
                .ok()
                .flatten()
                .unwrap_or(0),
        )
        .unwrap_or(0);
        ev(&format!(
            "lease_parent=ok container_hwnd=0x{:x}",
            self.container
        ));
        let parent = ParentSlotLease::new(lease.raw());
        let open_result = {
            let mut ops = dock_ops(&mut self.host, self.surface.clone(), self.instance);
            self.sessions
                .session_mut(self.session)
                .expect("session live")
                .open(&mut ops, &parent, SIZE, AttachMode::Docked)
        };
        match open_result {
            Ok(caps) => {
                if self.vst3_mode {
                    self.probe.parent_token.set(self.container as u64);
                    self.probe
                        .child_hwnd
                        .set(first_child_of(self.container as u64));
                }
                self.child = self.probe.child_hwnd.get();
                ev(&format!(
                    "open=ok caps_dock={:?} caps_close={:?}",
                    caps.dock, caps.close
                ));
                self.check(
                    "attach_parented",
                    self.child != 0 && parent_of(self.child) == self.container,
                    &format!(
                        "editor_hwnd=0x{:x} container=0x{:x}",
                        self.child, self.container
                    ),
                );
                self.check(
                    "attach_visible",
                    self.editor_visible(),
                    &format!("editor_rect={:?}", window_rect(self.child)),
                );
                self.enter(Stage::Attached);
            }
            Err(e) => {
                self.check("open", false, &format!("error={e:?}"));
                self.enter(Stage::Done);
            }
        }
    }

    /// Switches the layout's active tab, then enters `next`; a rejected
    /// update ends the run instead.
    fn switch_tab(&mut self, active: usize, next: Stage) {
        let result = self.driver.update_view(self.window, dock_view(active));
        self.check(
            "update_view",
            result.is_ok(),
            &format!("active_tab={active} error={result:?}"),
        );
        self.enter(if result.is_ok() { next } else { Stage::Done });
    }

    fn after_attached(&mut self) {
        screenshot(self.evidence_dir.as_deref(), "dock-attached", self.hwnd());
        self.switch_tab(TAB_OTHER, Stage::TabAway);
    }

    fn after_tab_away(&mut self) {
        let hidden = self.slot_visibility() == Some(SlotVisibility::Hidden);
        self.check(
            "tab_hide",
            hidden && !self.editor_visible(),
            &format!(
                "slot_visibility={:?} editor_visible={}",
                self.slot_visibility(),
                self.editor_visible()
            ),
        );
        screenshot(self.evidence_dir.as_deref(), "dock-tab-hidden", self.hwnd());
        self.switch_tab(TAB_EDITOR, Stage::TabBack);
    }

    fn after_tab_back(&mut self, drv: &mut dyn WindowDriver) {
        let shown = self.slot_visibility() == Some(SlotVisibility::Visible);
        self.check(
            "tab_show",
            shown && self.editor_visible(),
            &format!(
                "slot_visibility={:?} editor_visible={}",
                self.slot_visibility(),
                self.editor_visible()
            ),
        );
        let focus = self.surface.request_focus(self.slot.expect("slot"));
        self.check(
            "focus_container",
            focus.is_ok() && self.container != 0 && focus_hwnd() == self.container,
            &format!(
                "focus_hwnd=0x{:x} container=0x{:x}",
                focus_hwnd(),
                self.container
            ),
        );
        screenshot(self.evidence_dir.as_deref(), "dock-tab-shown", self.hwnd());
        self.rect_last = self.slot_rect();
        self.request_resize(drv, GROW_SIZE);
        self.enter(Stage::ResizeGrow);
    }

    /// Checks that the slot geometry changed after a resize and the
    /// editor stayed attached and visible.
    fn check_resize(&self, id: &str, label: &str) {
        let now = self.slot_rect();
        let scale = self.slot_spec().map_or(1.0, |s| s.geometry.scale_factor);
        let container_follows = match (now, window_rect(self.container)) {
            (Some((_, _, w, h)), Some((_, _, cw, ch))) => {
                (w * scale - f64::from(cw)).abs() < 1.0 && (h * scale - f64::from(ch)).abs() < 1.0
            }
            _ => false,
        };
        self.check(
            id,
            now.is_some() && now != self.rect_last && self.editor_visible() && container_follows,
            &format!(
                "slot_before={:?} slot_after={now:?} editor_rect={:?} container_rect={:?}",
                self.rect_last,
                window_rect(self.child),
                window_rect(self.container)
            ),
        );
        screenshot(self.evidence_dir.as_deref(), label, self.hwnd());
    }

    fn after_grow(&mut self, drv: &mut dyn WindowDriver) {
        self.check_resize("resize_grow_follows", "dock-resize-grow");
        self.rect_last = self.slot_rect();
        self.request_resize(drv, SHRINK_SIZE);
        self.enter(Stage::ResizeShrink);
    }

    fn after_shrink(&mut self) {
        self.check_resize("resize_shrink_follows", "dock-resize-shrink");
        self.close_and_teardown();
        self.enter(Stage::Done);
    }

    fn request_resize(&self, drv: &mut dyn WindowDriver, size: LogicalSize) {
        let (width, height) = (size.width, size.height);
        let result = drv.resize(self.window, size);
        ev(&format!(
            "resize_request=({width},{height}) result={}",
            if result.is_ok() { "ok" } else { "fail" }
        ));
    }

    /// Closes the session and tears the slot down; the close and the
    /// teardown are both observed on the OS state.
    fn close_and_teardown(&mut self) {
        let session = self.session;
        let close = {
            let mut ops = dock_ops(&mut self.host, self.surface.clone(), self.instance);
            self.sessions
                .session_mut(session)
                .expect("session live")
                .close(&mut ops)
        };
        let state = self.sessions.session_mut(session).map(|s| s.state());
        self.check(
            "close_session",
            close.is_ok() && state == Some(EditorSessionState::Closed) && !is_window(self.child),
            &format!("state={state:?} editor_alive={}", is_window(self.child)),
        );
        let slot = self.slot.expect("slot");
        let unbound = self
            .driver
            .unbind_native_slot(self.window, &ComponentKey(SLOT_KEY.to_string()));
        self.check(
            "unbind_native_slot",
            unbound.is_ok(),
            &format!("error={unbound:?}"),
        );
        let lease = self.lease.expect("lease minted");
        let released = self.surface.release_lease(lease);
        self.check(
            "release_lease",
            released.is_ok() && !is_window(self.container),
            &format!("container_alive={}", is_window(self.container)),
        );
        let released_slot = self.surface.release_slot(slot);
        self.check("release_slot", released_slot.is_ok(), "");
        match self.surface.drain_events() {
            Ok(events) => {
                let revoked = events
                    .iter()
                    .position(|e| matches!(e, SlotEvent::LeaseRevoked { .. }));
                let destroyed = events
                    .iter()
                    .position(|e| matches!(e, SlotEvent::Destroyed { .. }));
                self.check(
                    "slot_event_order",
                    matches!((revoked, destroyed), (Some(r), Some(d)) if r < d),
                    &format!("lease_revoked={revoked:?} destroyed={destroyed:?}"),
                );
                for e in &events {
                    ev(&format!("slot_event={e:?}"));
                }
            }
            Err(e) => self.check("slot_event_order", false, &format!("drain_error={e:?}")),
        }
    }

    fn tick(&mut self, drv: &mut dyn WindowDriver) {
        self.ticks += 1;
        match self.stage {
            Stage::WaitReady => {}
            Stage::AwaitProjection => self.await_projection(),
            _ if self.ticks < SETTLE_TICKS => {}
            Stage::Attached => self.after_attached(),
            Stage::TabAway => self.after_tab_away(),
            Stage::TabBack => self.after_tab_back(drv),
            Stage::ResizeGrow => self.after_grow(drv),
            Stage::ResizeShrink => self.after_shrink(),
            Stage::Done => {
                ev(&format!("summary failures={}", self.failures.get()));
                drv.exit();
            }
        }
    }

    /// The GPUI window handle that hosts the slot container.
    fn hwnd(&self) -> usize {
        if self.container == 0 {
            0
        } else {
            parent_of(self.container)
        }
    }
}

/// Entry point for the Dock scenario.
pub(super) fn run() {
    eprintln!("[poc] start os=windows scenario=native-dock-layout");
    let evidence_dir = std::env::var_os("RVF_POC_EVIDENCE_DIR").map(PathBuf::from);
    ev(&format!(
        "evidence_dir={}",
        evidence_dir.as_deref().map_or_else(
            || "unset (screenshots NotRun)".to_string(),
            |p| p.display().to_string()
        )
    ));

    let mut driver = GpuiDriver::new();
    eprintln!("[poc] capabilities={:?}", driver.capabilities());

    let mut layout = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    layout
        .mount(dock_view(TAB_EDITOR))
        .expect("mount dock layout");
    layout.drain_events();
    let window = driver
        .create_view_window(
            &WindowSpec {
                title: "ruviform — native dock layout".to_string(),
                logical_size: WINDOW_SIZE,
                ..WindowSpec::default()
            },
            ViewMount {
                host: layout,
                theme: None,
            },
        )
        .expect("create_view_window");
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
    let failures = Rc::new(Cell::new(0));

    let mut dock = Dock {
        driver: driver.clone(),
        surface: driver.surface(),
        window,
        host,
        sessions,
        session,
        instance,
        probe,
        vst3_mode,
        evidence_dir,
        failures: failures.clone(),
        slot: None,
        lease: None,
        container: 0,
        child: 0,
        rect_last: None,
        stage: Stage::WaitReady,
        ticks: 0,
    };

    driver
        .run(None, move |ev, drv| match ev {
            DriverEvent::Windowed {
                event: WindowLifecycle::Ready,
                window: w,
            } if w == dock.window && matches!(dock.stage, Stage::WaitReady) => {
                eprintln!("[poc] ready");
                dock.on_ready(drv);
            }
            DriverEvent::Windowed {
                event: WindowLifecycle::CloseRequested,
                window: w,
            } if w == dock.window => {
                let _ = drv.respond_close(w, CloseDecision::Approve);
            }
            DriverEvent::Tick => dock.tick(drv),
            _ => {}
        })
        .expect("run");
    eprintln!("[poc] exited failures={}", failures.get());
    if failures.get() > 0 {
        std::process::exit(1);
    }
}
