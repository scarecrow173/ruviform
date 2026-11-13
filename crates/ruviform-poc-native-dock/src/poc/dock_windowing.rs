//! Native Dock windowing scenario (Issue #388 slice 3): ancestor
//! clip/overhang, keyboard/focus routing, and `DockWorkspace`
//! detach→reattach 2-phase moves against a real plugin editor on a
//! real GPUI window.
//!
//! Selected with `RVF_POC_SCENARIO=dock-windowing`. Clip evidence
//! combines the layout-driven path (window resized below the slot's
//! `min_size`, so the renderer projects a real clip) with a direct
//! `set_clip` port call; key routing is observed through a
//! `WH_GETMESSAGE` hook scoped to this thread's queue plus a real
//! mouse click; re-dock drives the real `DockWorkspace` model while
//! `update_view` suppresses/restores the `NativeSlot` component.
//!
//! Reports `[poc] check=…` lines only — the capability-matrix verdict
//! is taken from the evidence report, not this process exit alone.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use ruviform_plugin_core::ParentLease;
use ruviform_plugin_host::{
    AttachMode, EditorOpError, EditorSessionState, InProcessEditorOps, InProcessHost,
    ParentSlotLease, PluginEditorSessionId, PluginEditorSessions, PluginHandle,
};
use ruviform_ui::dock::{DockEvent, DockTarget, DockWorkspace};
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

use super::probe::{create_probe_child, destroy_editor, probe_message_counts};
use super::{
    ProbeShared, SIZE, ancestor_chain, click_at, client_size, clip_box, descendants_of,
    drain_msg_log, ev, first_child_of, focus_hwnd, hook_messages, instantiate, is_visible,
    is_window, parent_of, printwindow_pixel, region_box, screen_pixel, send_key, set_focus,
    unhook_messages, window_from_point, window_pixel, window_rect, window_thread_id, window_tree,
};

const SLOT_KEY: &str = "dockw.editor";
const TAB_EDITOR: usize = 0;

/// The view instance the dock model and the mounted layout share.
fn view_id() -> ViewInstanceId {
    ViewInstanceId::new(1).expect("nonzero")
}
const SETTLE_TICKS: u32 = 20;
const PROJECTION_TIMEOUT: u32 = 600;
const WINDOW_SIZE: LogicalSize = LogicalSize {
    width: 1000,
    height: 720,
};
/// Below the slot's `min_size` (400x300) in both axes once the tab
/// header is subtracted — forces the laid-out element to overhang
/// the viewport so the projection emits a real clip.
const CLIP_SIZE: LogicalSize = LogicalSize {
    width: 380,
    height: 300,
};
/// Virtual-key codes for the injected keys ('A'/'B'/'C').
const VK_A: u8 = 0x41;
const VK_B: u8 = 0x42;
const VK_C: u8 = 0x43;

/// Ordered scenario stages; each non-WaitReady stage waits
/// `SETTLE_TICKS` ticks before its observation runs so OS-level
/// effects flush through the message queue.
#[derive(Copy, Clone, Debug)]
enum Stage {
    WaitReady,
    AwaitProjection,
    Attached,
    ClipOverhang,
    ClipRestore,
    ClipDirectCheck,
    ClipDirectClear,
    FocusContainer,
    KeyContainerCheck,
    ClickPlugin,
    KeyPluginCheck,
    KeyBackCheck,
    RedockMove,
    RedockDetach,
    RedockDetachedCheck,
    RedockAttachedCheck,
    Done,
}

/// Panel → Tabs layout; the editor tab holds the `NativeSlot` while
/// `with_slot` is true and an inert panel while detached.
fn windowing_view(with_slot: bool, active: usize) -> ViewSpec {
    let editor_child = if with_slot {
        ComponentSpec::NativeSlot(NativeSlotSpec {
            key: ComponentKey(SLOT_KEY.to_string()),
            min_size: Some([SIZE.width, SIZE.height]),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    } else {
        ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("dockw.editor.panel".to_string()),
            title: None,
            children: vec![],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    };
    let other = ComponentSpec::Panel(PanelSpec {
        key: ComponentKey("dockw.other".to_string()),
        title: None,
        children: vec![],
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    });
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("dockw.root".to_string()),
            title: None,
            children: vec![ComponentSpec::Tabs(TabsSpec {
                key: ComponentKey("dockw.tabs".to_string()),
                active,
                tabs: vec![
                    TabSpec {
                        key: ComponentKey("tab.editor".to_string()),
                        title: "Editor".to_string(),
                        child: Box::new(editor_child),
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

/// Redeemer composition against the driver's canonical surface.
fn windowing_ops(
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
    if let Err(e) = std::fs::create_dir_all(dir) {
        ev(&format!("screenshot={label} result=fail reason=mkdir {e}"));
        return;
    }
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

/// Classifies drained `(hwnd, message)` records into "container got a
/// key" / "plugin tree got a key" for WM_KEYDOWN/WM_CHAR only.
fn key_targets(log: &[(usize, u32)], container: usize, tree: &[usize]) -> (bool, bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{WM_CHAR, WM_KEYDOWN};
    let mut container_got = false;
    let mut plugin_got = false;
    for &(hwnd, msg) in log {
        if msg != WM_KEYDOWN && msg != WM_CHAR {
            continue;
        }
        if hwnd == container {
            container_got = true;
        }
        if tree.contains(&hwnd) {
            plugin_got = true;
        }
    }
    (container_got, plugin_got)
}

struct Windowing {
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
    /// Real `DockWorkspace` model driving the detach→reattach flow.
    ws: DockWorkspace,
    slot: Option<SurfaceSlotId>,
    lease: Option<SlotLease>,
    /// Scratch slot used only for the busy/lease handshake.
    scratch: Option<SurfaceSlotId>,
    /// Slot container HWND (the parent the editor attaches under).
    container: usize,
    child: usize,
    /// Plugin window tree (editor + descendants) for key attribution.
    plugin_tree: Vec<usize>,
    /// Logical slot rect snapshot for clip expectation math.
    rect_last: Option<LogicalRectTuple>,
    stage: Stage,
    ticks: u32,
}

impl Windowing {
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

    fn scale(&self) -> f64 {
        self.slot_spec().map_or(1.0, |s| s.geometry.scale_factor)
    }

    fn editor_visible(&self) -> bool {
        is_visible(self.child)
    }

    /// The GPUI window handle that hosts the slot container.
    fn hwnd(&self) -> usize {
        if self.container == 0 {
            0
        } else {
            parent_of(self.container)
        }
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
            view: view_id(),
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
    /// for the bound slot.
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

    /// Leases the projected slot and opens the editor session.
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
            let mut ops = windowing_ops(&mut self.host, self.surface.clone(), self.instance);
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
                self.plugin_tree = window_tree(self.child);
                ev(&format!(
                    "open=ok caps_dock={:?} caps_close={:?} plugin_tree={}",
                    caps.dock,
                    caps.close,
                    self.plugin_tree.len()
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
                // Scope the key-observation hook to this thread's queue.
                let hwnd = self.hwnd();
                let tid = window_thread_id(hwnd);
                let hooked = hook_messages(tid);
                ev(&format!(
                    "msg_hook={} thread_id={tid} window_hwnd=0x{hwnd:x}",
                    if hooked { "installed" } else { "failed" }
                ));
                self.check("msg_hook", hooked, "");
                self.enter(Stage::Attached);
            }
            Err(e) => {
                self.check("open", false, &format!("error={e:?}"));
                self.enter(Stage::Done);
            }
        }
    }

    /// Compositing diagnostics: what the child painted into its own
    /// DC, and which HWND `WindowFromPoint` reports topmost at the
    /// editor's center — distinguishes "child not painting" from
    /// "child painting but covered".
    fn compositor_probe(&self, tag: &str) {
        if let Some((ex, ey, ew, eh)) = window_rect(self.child) {
            let (cx, cy) = (ex + ew / 2, ey + eh / 2);
            // Sample near the child's top-left — inside every clip
            // region this scenario applies — plus the geometric
            // center for contrast.
            let (own_tl, own_tl_dcnull) = window_pixel(self.child, 8, 8);
            let (own_c, own_c_dcnull) = window_pixel(self.child, ew / 2, eh / 2);
            let printed = printwindow_pixel(self.child);
            let top = window_from_point(cx, cy);
            let siblings: Vec<usize> = descendants_of(self.container)
                .into_iter()
                .filter(|h| *h != self.child && window_tree(*h).len() == 1)
                .collect();
            ev(&format!(
                "compositor[{tag}] editor_rect=({ex},{ey},{ew},{eh}) center=({cx},{cy}) \
                 own_tl=0x{own_tl:08x}(dcnull={own_tl_dcnull}) \
                 own_c=0x{own_c:08x}(dcnull={own_c_dcnull}) print_px=0x{printed:08x} \
                 clip={:?} clip_c={:?} region={:?} gpui_region={:?} siblings={siblings:?} \
                 topmost=0x{top:x} child=0x{:x} container=0x{:x} container_rect={:?} \
                 gpui=0x{:x} gpui_rect={:?}",
                clip_box(self.child),
                clip_box(self.container),
                region_box(self.container),
                region_box(self.hwnd()),
                self.child,
                self.container,
                window_rect(self.container),
                self.hwnd(),
                window_rect(self.hwnd())
            ));
            if tag == "attached" {
                for line in ancestor_chain(self.child) {
                    ev(&format!("ancestor {line}"));
                }
                // Sibling probe: a second probe-class child of the
                // same container — does ANY child HWND composite?
                if let Ok(sib) = create_probe_child(self.container as u64, 40, 40, 160, 120) {
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    if let Some((sx, sy, sw, sh)) = window_rect(sib) {
                        let px = screen_pixel(sx + sw / 2, sy + sh / 2);
                        ev(&format!(
                            "sibling_probe hwnd=0x{sib:x} rect=({sx},{sy},{sw},{sh}) \
                             screen_px=0x{px:08x} clip={:?} topmost=0x{:x}",
                            clip_box(sib),
                            window_from_point(sx + sw / 2, sy + sh / 2)
                        ));
                    }
                    destroy_editor(sib);
                }
            }
        }
    }

    fn after_attached(&mut self, drv: &mut dyn WindowDriver) {
        self.compositor_probe("attached");
        screenshot(self.evidence_dir.as_deref(), "dockw-attached", self.hwnd());
        let result = drv.resize(self.window, CLIP_SIZE);
        ev(&format!(
            "resize_request=clip_overhang result={}",
            if result.is_ok() { "ok" } else { "fail" }
        ));
        self.enter(Stage::ClipOverhang);
    }

    /// The window is now smaller than the slot's `min_size`: the
    /// projection should have pushed `element ∩ viewport` as the
    /// clip. Verify the OS window region matches the expectation.
    fn after_clip_overhang(&mut self, drv: &mut dyn WindowDriver) {
        let Some((sx, sy, sw, sh)) = self.slot_rect() else {
            self.check("clip_overhang", false, "slot_rect=unavailable");
            self.enter(Stage::Done);
            return;
        };
        let scale = self.scale();
        let viewport =
            client_size(self.hwnd()).map(|(w, h)| (f64::from(w) / scale, f64::from(h) / scale));
        let Some((vw, vh)) = viewport else {
            self.check("clip_overhang", false, "client_size=unavailable");
            self.enter(Stage::Done);
            return;
        };
        // Expected clip: slot rect ∩ (0,0,vw,vh), mapped to
        // container-local physical px — exactly what `clip_local`
        // computes for `SetWindowRgn`.
        let vx = sx.max(0.0);
        let vy = sy.max(0.0);
        let vr = (sx + sw).min(vw);
        let vb = (sy + sh).min(vh);
        let expected = (
            ((vx - sx) * scale).round() as i32,
            ((vy - sy) * scale).round() as i32,
            ((vr - sx) * scale).round() as i32,
            ((vb - sy) * scale).round() as i32,
        );
        let region = region_box(self.container);
        let overhang = sw > vw || sh > vh || sx < 0.0 || sy < 0.0;
        let region_ok = match region {
            Some((l, t, r, b)) => {
                (l - expected.0).abs() <= 2
                    && (t - expected.1).abs() <= 2
                    && (r - expected.2).abs() <= 2
                    && (b - expected.3).abs() <= 2
            }
            None => false,
        };
        self.check(
            "clip_overhang_region",
            overhang && region_ok,
            &format!(
                "slot_rect=({sx:.1},{sy:.1},{sw:.1},{sh:.1}) viewport_logical=({vw:.1},{vh:.1}) \
                 expected_region={expected:?} actual_region={region:?} scale={scale}"
            ),
        );
        self.check(
            "clip_overhang_editor_alive",
            self.editor_visible() && parent_of(self.child) == self.container,
            &format!(
                "editor_visible={} parent_ok={}",
                self.editor_visible(),
                parent_of(self.child) == self.container
            ),
        );
        screenshot(
            self.evidence_dir.as_deref(),
            "dockw-clip-overhang",
            self.hwnd(),
        );
        let result = drv.resize(self.window, WINDOW_SIZE);
        ev(&format!(
            "resize_request=restore result={}",
            if result.is_ok() { "ok" } else { "fail" }
        ));
        self.enter(Stage::ClipRestore);
    }

    /// Window restored → the projection should push `clip=None` and
    /// the container's window region should be gone.
    fn after_clip_restore(&mut self) {
        let region = region_box(self.container);
        self.check(
            "clip_cleared",
            region.is_none() && self.editor_visible(),
            &format!(
                "region={region:?} editor_visible={} slot_rect={:?}",
                self.editor_visible(),
                self.slot_rect()
            ),
        );
        // Direct port clip: left half of the slot in window coords.
        if let Some((sx, sy, sw, sh)) = self.slot_rect() {
            let clip = LogicalRect {
                x: sx,
                y: sy,
                width: sw * 0.5,
                height: sh,
            };
            let result = self.surface.set_clip(self.slot.expect("slot"), Some(clip));
            ev(&format!(
                "set_clip_direct={:?} clip={clip:?}",
                result.is_ok()
            ));
            self.rect_last = Some((sx, sy, sw, sh));
        }
        self.enter(Stage::ClipDirectCheck);
    }

    /// The direct `set_clip` should show up as a container-local
    /// region covering the left half, and pixels outside the clip
    /// should no longer show the editor.
    fn after_clip_direct_check(&mut self) {
        let scale = self.scale();
        let Some((_, _, sw, sh)) = self.rect_last else {
            self.check("clip_direct", false, "slot_rect=unavailable");
            self.enter(Stage::ClipDirectClear);
            return;
        };
        let expected = (
            0,
            0,
            (sw * 0.5 * scale).round() as i32,
            (sh * scale).round() as i32,
        );
        let region = region_box(self.container);
        let region_ok = match region {
            Some((l, t, r, b)) => {
                (l - expected.0).abs() <= 2
                    && (t - expected.1).abs() <= 2
                    && (r - expected.2).abs() <= 2
                    && (b - expected.3).abs() <= 2
            }
            None => false,
        };
        // Hit-test evidence: `WindowFromPoint` respects the container's
        // window region — inside the clip the editor answers, outside
        // it the GPUI frame does. Sample points straddle the clip
        // boundary in *container* coordinates (the clip covers the
        // container-local left half); child-quartile sampling breaks
        // for plugins whose editor is smaller than the slot. Pixel
        // sampling is diagnostic only: GPUI binds its DirectComposition
        // target with `topmost=true`, so child HWNDs never composite to
        // the screen and no screen-pixel difference can be observed.
        let Some((cx0, cy0, cw, ch)) = window_rect(self.container) else {
            self.check("clip_direct_hit_test", false, "container_rect=unavailable");
            self.enter(Stage::ClipDirectClear);
            return;
        };
        let inside = (cx0 + cw / 4, cy0 + ch / 2);
        let outside = (cx0 + cw * 3 / 4, cy0 + ch / 2);
        let center = (cx0 + cw / 2, cy0 + ch / 2);
        let (pin, pout, pcen) = (
            screen_pixel(inside.0, inside.1),
            screen_pixel(outside.0, outside.1),
            screen_pixel(center.0, center.1),
        );
        let (top_in, top_out) = (
            window_from_point(inside.0, inside.1),
            window_from_point(outside.0, outside.1),
        );
        ev(&format!(
            "clip_pixels container_rect=({cx0},{cy0},{cw},{ch}) inside=({},{})=0x{pin:08x} \
             outside=({},{})=0x{pout:08x} center=({},{})=0x{pcen:08x} \
             topmost inside=0x{top_in:x} outside=0x{top_out:x} child=0x{:x} gpui=0x{:x}",
            inside.0,
            inside.1,
            outside.0,
            outside.1,
            center.0,
            center.1,
            self.child,
            self.hwnd()
        ));
        self.check(
            "clip_direct_region",
            region_ok,
            &format!("expected_region={expected:?} actual_region={region:?} scale={scale}"),
        );
        let in_plugin_tree = window_tree(self.child).contains(&top_in);
        self.check(
            "clip_direct_hit_test",
            in_plugin_tree && top_out == self.hwnd(),
            &format!(
                "topmost_inside=0x{top_in:x} topmost_outside=0x{top_out:x} \
                 child=0x{:x} gpui=0x{:x} (dcomp_topmost covers child pixels)",
                self.child,
                self.hwnd()
            ),
        );
        self.compositor_probe("clip-direct");
        screenshot(
            self.evidence_dir.as_deref(),
            "dockw-clip-direct",
            self.hwnd(),
        );
        let cleared = self.surface.set_clip(self.slot.expect("slot"), None);
        ev(&format!("set_clip_clear={:?}", cleared.is_ok()));
        self.enter(Stage::ClipDirectClear);
    }

    /// The cleared clip must remove the container's window region.
    fn after_clip_direct_clear(&mut self) {
        self.check(
            "clip_direct_cleared",
            region_box(self.container).is_none(),
            &format!("region={:?}", region_box(self.container)),
        );
        self.enter(Stage::FocusContainer);
    }

    /// `request_focus` must land on the slot container, not the
    /// editor — the container/plugin focus boundary.
    fn after_focus_container(&mut self) {
        let slot = self.slot.expect("slot");
        let focus = self.surface.request_focus(slot);
        self.check(
            "focus_container",
            focus.is_ok() && self.container != 0 && focus_hwnd() == self.container,
            &format!(
                "focus={focus:?} focus_hwnd=0x{:x} container=0x{:x}",
                focus_hwnd(),
                self.container
            ),
        );
        // Baseline: a key sent while the container holds focus must be
        // delivered to the container HWND, never into the plugin tree.
        drain_msg_log();
        send_key(VK_A);
        self.enter(Stage::KeyContainerCheck);
    }

    /// Drain the message log and attribute the injected key.
    fn after_key_container_check(&mut self) {
        let log = drain_msg_log();
        let (container_got, plugin_got) = key_targets(&log, self.container, &self.plugin_tree);
        let probe_counts = probe_message_counts();
        ev(&format!(
            "key_container log={} container_got={container_got} plugin_got={plugin_got} \
             probe_counts={probe_counts:?} focus_hwnd=0x{:x}",
            log.len(),
            focus_hwnd()
        ));
        self.check(
            "key_to_container_not_plugin",
            container_got && !plugin_got,
            &format!("container_got={container_got} plugin_got={plugin_got}"),
        );
        // Real click at the editor's center — the user path that moves
        // keyboard focus into the plugin window tree.
        if let Some((x, y, w, h)) = window_rect(self.child) {
            click_at(x + w / 2, y + h / 2);
            ev(&format!("click_at=({},{})", x + w / 2, y + h / 2));
        } else {
            ev("click_at=skip reason=no_editor_rect");
        }
        self.enter(Stage::ClickPlugin);
    }

    /// After the click settles, focus should sit inside the plugin
    /// window tree; `SetFocus` is the recorded fallback.
    fn after_plugin_focus_check(&mut self) {
        let focus = focus_hwnd();
        let in_tree = self.plugin_tree.contains(&focus);
        let path = if in_tree {
            "click"
        } else {
            let prev = set_focus(self.child);
            ev(&format!(
                "focus_fallback=SetFocus prev=0x{prev:x} child=0x{:x}",
                self.child
            ));
            "setfocus"
        };
        let focus_now = focus_hwnd();
        self.check(
            "click_focuses_plugin",
            self.plugin_tree.contains(&focus_now),
            &format!(
                "path={path} focus_hwnd=0x{focus_now:x} plugin_tree={}",
                self.plugin_tree.len()
            ),
        );
        drain_msg_log();
        send_key(VK_B);
        self.enter(Stage::KeyPluginCheck);
    }

    /// The key sent while the plugin tree holds focus must be
    /// delivered to a plugin HWND — the routing evidence.
    fn after_key_plugin_check(&mut self) {
        let log = drain_msg_log();
        let (container_got, plugin_got) = key_targets(&log, self.container, &self.plugin_tree);
        let probe_counts = probe_message_counts();
        ev(&format!(
            "key_plugin log={} container_got={container_got} plugin_got={plugin_got} \
             probe_counts={probe_counts:?} focus_hwnd=0x{:x}",
            log.len(),
            focus_hwnd()
        ));
        self.check(
            "key_reaches_plugin",
            plugin_got && !container_got,
            &format!("container_got={container_got} plugin_got={plugin_got}"),
        );
        // Boundary back: request_focus must return focus to the
        // container and keep keys out of the plugin tree.
        let slot = self.slot.expect("slot");
        let focus = self.surface.request_focus(slot);
        ev(&format!(
            "focus_return={focus:?} focus_hwnd=0x{:x}",
            focus_hwnd()
        ));
        drain_msg_log();
        send_key(VK_C);
        self.enter(Stage::KeyBackCheck);
    }

    /// Same attribution as `key_to_container_not_plugin`, after the
    /// focus round-trip.
    fn after_key_back_check(&mut self) {
        let log = drain_msg_log();
        let (container_got, plugin_got) = key_targets(&log, self.container, &self.plugin_tree);
        self.check(
            "key_boundary_return",
            container_got && !plugin_got && focus_hwnd() == self.container,
            &format!(
                "container_got={container_got} plugin_got={plugin_got} focus_hwnd=0x{:x}",
                focus_hwnd()
            ),
        );
        self.enter(Stage::RedockMove);
    }

    /// Real `DockWorkspace` 2-phase move + the slot busy handshake:
    /// attach the view, `begin_move`, mark the scratch slot busy,
    /// prove `lease_parent` refuses with `SlotBusy`, `commit_move`,
    /// clear busy, prove the lease succeeds.
    fn after_redock_move(&mut self) {
        let view = view_id();
        self.ws.register_window(self.window);
        let attach = self.ws.attach_view(view, DockTarget::Window(self.window));
        self.check(
            "redock_ws_attach",
            attach.is_ok() && self.ws.owner_of(view) == Some(self.window),
            &format!("attach={attach:?} owner={:?}", self.ws.owner_of(view)),
        );

        // Scratch slot for the busy handshake — declared, never
        // painted, leased only inside this stage. It needs the same
        // attach capability as the main slot or `lease_parent`
        // refuses for the wrong reason.
        let scale = self.scale();
        let scratch_spec = SlotSpec {
            window: self.window,
            view: ViewInstanceId::new(2).expect("nonzero"),
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
            capabilities: SlotCapabilities {
                attach_parent: Support::Supported,
                ..SlotCapabilities::default()
            },
        };
        let scratch = match self.surface.declare_slot(scratch_spec) {
            Ok(slot) => slot,
            Err(e) => {
                self.check("redock_scratch_declared", false, &format!("error={e}"));
                self.enter(Stage::Done);
                return;
            }
        };
        self.scratch = Some(scratch);
        self.check(
            "redock_scratch_declared",
            true,
            &format!("index={}", scratch.index()),
        );

        let token = self.ws.begin_move(view, DockTarget::Window(self.window));
        self.check(
            "redock_begin_move",
            token.is_ok() && self.ws.is_transferring(view),
            &format!(
                "begin={token:?} transferring={}",
                self.ws.is_transferring(view)
            ),
        );
        let Ok(token) = token else {
            self.enter(Stage::Done);
            return;
        };

        // Busy window: a new lease on the busy slot must be refused.
        let marked = self.surface.mark_busy(scratch);
        let busy_lease = self.surface.lease_parent(scratch);
        let busy_refused = matches!(busy_lease, Err(SlotError::SlotBusy));
        self.check(
            "redock_busy_refuses_lease",
            marked.is_ok() && busy_refused,
            &format!("mark_busy={marked:?} lease={busy_lease:?}"),
        );

        let report = self.ws.commit_move(token);
        self.check(
            "redock_commit_move",
            report.as_ref().is_ok_and(|r| r.to == self.window),
            &format!("commit={report:?}"),
        );

        let cleared = self.surface.clear_busy(scratch);
        let lease2 = self.surface.lease_parent(scratch);
        self.check(
            "redock_busy_cleared_lease",
            cleared.is_ok() && lease2.is_ok(),
            &format!("clear_busy={cleared:?} lease={}", lease2.is_ok()),
        );
        if let Ok(lease2) = lease2 {
            let _ = self.surface.release_lease(lease2);
        }
        let _ = self.surface.release_slot(scratch);
        self.scratch = None;

        let events = self.ws.drain_events();
        let prepared = events
            .iter()
            .any(|e| matches!(e, DockEvent::MovePrepared { view: v } if *v == view));
        let committed = events.iter().any(|e| {
            matches!(e, DockEvent::MoveCommitted { view: v, to, .. } if *v == view && *to == self.window)
        });
        self.check(
            "redock_move_events",
            prepared && committed,
            &format!("events={events:?}"),
        );
        self.enter(Stage::RedockDetach);
    }

    /// `detach_view` → detached pool (the float candidate), mirrored
    /// on the GUI by removing the `NativeSlot` from the view spec.
    fn after_redock_detach(&mut self) {
        let detached = self.ws.detach_view(view_id());
        self.check(
            "redock_detach",
            detached.is_ok() && self.ws.is_detached(view_id()),
            &format!(
                "detach={detached:?} is_detached={}",
                self.ws.is_detached(view_id())
            ),
        );
        let updated = self
            .driver
            .update_view(self.window, windowing_view(false, TAB_EDITOR));
        self.check(
            "redock_update_view_detach",
            updated.is_ok(),
            &format!("{updated:?}"),
        );
        self.enter(if updated.is_ok() {
            Stage::RedockDetachedCheck
        } else {
            Stage::Done
        });
    }

    /// While detached the slot must be hidden and the editor
    /// invisible, yet the HWND/container/lease must survive — then
    /// reattach restores the same surface.
    fn after_redock_detached_check(&mut self) {
        let hidden = self.slot_visibility() == Some(SlotVisibility::Hidden);
        let alive = is_window(self.child) && parent_of(self.child) == self.container;
        self.check(
            "redock_detach_surface_survives",
            hidden && !self.editor_visible() && alive,
            &format!(
                "slot_visibility={:?} editor_visible={} child_alive={} parent_ok={}",
                self.slot_visibility(),
                self.editor_visible(),
                is_window(self.child),
                parent_of(self.child) == self.container
            ),
        );
        screenshot(self.evidence_dir.as_deref(), "dockw-detached", self.hwnd());
        let attached = self
            .ws
            .attach_view(view_id(), DockTarget::Window(self.window));
        self.check(
            "redock_reattach",
            attached.is_ok() && self.ws.owner_of(view_id()) == Some(self.window),
            &format!(
                "attach={attached:?} owner={:?}",
                self.ws.owner_of(view_id())
            ),
        );
        let updated = self
            .driver
            .update_view(self.window, windowing_view(true, TAB_EDITOR));
        self.check(
            "redock_update_view_attach",
            updated.is_ok(),
            &format!("{updated:?}"),
        );
        self.enter(if attached.is_ok() && updated.is_ok() {
            Stage::RedockAttachedCheck
        } else {
            Stage::Done
        });
    }

    /// Reattached: the same editor HWND must be visible again under
    /// the same container — the surface survived the whole transfer.
    fn after_redock_attached_check(&mut self) {
        let shown = self.slot_visibility() == Some(SlotVisibility::Visible);
        let same = is_window(self.child) && parent_of(self.child) == self.container;
        self.check(
            "redock_reattach_same_surface",
            shown && self.editor_visible() && same,
            &format!(
                "slot_visibility={:?} editor_visible={} editor_hwnd=0x{:x} parent_ok={}",
                self.slot_visibility(),
                self.editor_visible(),
                self.child,
                parent_of(self.child) == self.container
            ),
        );
        self.compositor_probe("reattached");
        screenshot(
            self.evidence_dir.as_deref(),
            "dockw-reattached",
            self.hwnd(),
        );
        self.close_and_teardown();
        self.enter(Stage::Done);
    }

    /// Closes the session and tears the slot down; same order as the
    /// dock layout scenario.
    fn close_and_teardown(&mut self) {
        unhook_messages();
        let session = self.session;
        let close = {
            let mut ops = windowing_ops(&mut self.host, self.surface.clone(), self.instance);
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
                // Order is judged per slot — the scratch slot's own
                // Destroyed must not satisfy this slot's pair.
                let revoked = events.iter().position(
                    |e| matches!(e, SlotEvent::LeaseRevoked { slot: s, .. } if *s == slot),
                );
                let destroyed = events
                    .iter()
                    .position(|e| matches!(e, SlotEvent::Destroyed { slot: s, .. } if *s == slot));
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
            Stage::Attached => self.after_attached(drv),
            Stage::ClipOverhang => self.after_clip_overhang(drv),
            Stage::ClipRestore => self.after_clip_restore(),
            Stage::ClipDirectCheck => self.after_clip_direct_check(),
            Stage::ClipDirectClear => self.after_clip_direct_clear(),
            Stage::FocusContainer => self.after_focus_container(),
            Stage::KeyContainerCheck => self.after_key_container_check(),
            Stage::ClickPlugin => self.after_plugin_focus_check(),
            Stage::KeyPluginCheck => self.after_key_plugin_check(),
            Stage::KeyBackCheck => self.after_key_back_check(),
            Stage::RedockMove => self.after_redock_move(),
            Stage::RedockDetach => self.after_redock_detach(),
            Stage::RedockDetachedCheck => self.after_redock_detached_check(),
            Stage::RedockAttachedCheck => self.after_redock_attached_check(),
            Stage::Done => {
                ev(&format!("summary failures={}", self.failures.get()));
                drv.exit();
            }
        }
    }
}

/// Entry point for the windowing scenario.
pub(super) fn run() {
    eprintln!("[poc] start os=windows scenario=native-dock-windowing");
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

    let mut layout = ViewHost::new(view_id(), AttachEpoch::new(1));
    layout
        .mount(windowing_view(true, TAB_EDITOR))
        .expect("mount windowing layout");
    layout.drain_events();
    let window = driver
        .create_view_window(
            &WindowSpec {
                title: "ruviform — native dock windowing".to_string(),
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

    let mut w = Windowing {
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
        ws: DockWorkspace::new(),
        slot: None,
        lease: None,
        scratch: None,
        container: 0,
        child: 0,
        plugin_tree: Vec::new(),
        rect_last: None,
        stage: Stage::WaitReady,
        ticks: 0,
    };

    driver
        .run(None, move |ev, drv| match ev {
            DriverEvent::Windowed {
                event: WindowLifecycle::Ready,
                window: wid,
            } if wid == w.window && matches!(w.stage, Stage::WaitReady) => {
                eprintln!("[poc] ready");
                w.on_ready(drv);
            }
            DriverEvent::Windowed {
                event: WindowLifecycle::CloseRequested,
                window: wid,
            } if wid == w.window => {
                let _ = drv.respond_close(wid, CloseDecision::Approve);
            }
            DriverEvent::Tick => w.tick(drv),
            _ => {}
        })
        .expect("run");
    eprintln!("[poc] exited failures={}", failures.get());
    if failures.get() > 0 {
        std::process::exit(1);
    }
}
