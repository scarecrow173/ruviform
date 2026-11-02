//! Dock workspace contract tests (Issue #182) — a fake host drives
//! the cross-window view-move contract end to end through the public
//! API, proving:
//!
//! - a view moves across windows in two phases — destination prepared
//!   first, then ownership switches atomically (`begin_move` →
//!   webview-ready → `commit_move`);
//! - window-creation/webview failure rolls back to the source dock —
//!   the view never left it;
//! - detach → reattach preserves the logical `ViewInstanceId` (view
//!   state rides along, keyed by id on the consumer side — DOM
//!   transfer is never promised);
//! - duplicate view ownership is rejected across every window;
//! - a window close applies the consumer's explicit disposition —
//!   nothing silently tears down views or jobs;
//! - layout snapshots round-trip: geometry, monitor signatures, dock
//!   trees, detached pool — with off-screen/missing-monitor
//!   correction and safe-fallback restore.
//!
//! **RT contract:** non-RT only.

use std::collections::HashMap;

use ruviform_ui::{
    CloseDecision, DockEvent, DockNode, DockTarget, DockWorkspace, DropZone, LayoutSnapshot,
    MonitorHint, MonitorInfo, UiError, ViewInstanceId, WindowCloseDisposition, WindowConfig,
    WindowGeometry, WindowHandle, WindowId,
};

// ---------- fake host ----------

/// Minimal host implementing [`WindowHandle`]; `fail_creates` makes
/// the next `create_window` calls fail, modeling native-window or
/// webview creation errors (IPC init failure, OOM, display gone).
struct FakeHost {
    next: u64,
    ready: HashMap<WindowId, bool>,
    fail_creates: usize,
    created_configs: HashMap<WindowId, WindowConfig>,
}

impl FakeHost {
    fn new() -> Self {
        Self {
            next: 1,
            ready: HashMap::new(),
            fail_creates: 0,
            created_configs: HashMap::new(),
        }
    }

    /// The webview finished loading → the destination is prepared.
    fn mark_ready(&mut self, w: WindowId) {
        *self.ready.get_mut(&w).expect("window exists") = true;
    }
}

impl WindowHandle for FakeHost {
    fn eval(&mut self, _window: WindowId, _script: &str) {}

    fn resize(&mut self, window: WindowId, _w: u32, _h: u32) -> Result<(), UiError> {
        if self.ready.contains_key(&window) {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn focus(&mut self, window: WindowId, _focused: bool) -> Result<(), UiError> {
        if self.ready.contains_key(&window) {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn create_window(&mut self, config: &WindowConfig) -> Result<WindowId, UiError> {
        if self.fail_creates > 0 {
            self.fail_creates -= 1;
            return Err(UiError::host("native window creation failed".to_string()));
        }
        let id = WindowId::new(self.next).expect("nonzero");
        self.next += 1;
        self.ready.insert(id, false);
        self.created_configs.insert(id, config.clone());
        Ok(id)
    }

    fn request_close(&mut self, _window: WindowId) {}

    fn respond_close(
        &mut self,
        _window: WindowId,
        _decision: CloseDecision,
    ) -> Result<(), UiError> {
        Ok(())
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        if self.ready.remove(&window).is_some() {
            Ok(())
        } else {
            Err(UiError::host(format!("no window {window}")))
        }
    }

    fn windows(&self) -> Vec<WindowId> {
        self.ready.keys().copied().collect()
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        vec![MonitorInfo {
            name: Some("fake-monitor".to_string()),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale_factor: 1.0,
        }]
    }

    fn exit(&mut self) {}
}

// ---------- helpers ----------

fn vid(raw: u64) -> ViewInstanceId {
    ViewInstanceId::new(raw).expect("nonzero")
}

fn geometry(x: i32, y: i32, w: u32, h: u32) -> WindowGeometry {
    WindowGeometry {
        x,
        y,
        width: w,
        height: h,
        monitor: None,
    }
}

fn view_ids(ws: &DockWorkspace, w: WindowId) -> Vec<ViewInstanceId> {
    ws.tree(w).map(|t| t.views().collect()).unwrap_or_default()
}

// ---------- tests ----------

/// Two-phase move: `begin_move` locks the view while the destination
/// window + webview is being prepared; `commit_move` performs the
/// atomic ownership switch. State save/restore keyed by the stable
/// `ViewInstanceId` rides across — the *same logical view* arrives.
#[test]
fn two_phase_move_prepares_destination_then_switches_ownership() {
    let mut host = FakeHost::new();
    let mut ws = DockWorkspace::new();

    let w1 = host.create_window(&WindowConfig::default()).unwrap();
    let w2 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(2), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(3), DockTarget::Window(w2)).unwrap();

    // Consumer-side view state keyed by the logical id.
    let mut view_state: HashMap<ViewInstanceId, String> = HashMap::new();
    view_state.insert(vid(1), "scroll=42;filter=on".to_string());

    // Phase 1: begin — view is locked for editing.
    let token = ws
        .begin_move(vid(1), DockTarget::Window(w2))
        .expect("begin move");
    assert!(ws.is_transferring(vid(1)));
    assert!(matches!(
        ws.detach_view(vid(1)),
        Err(ruviform_ui::DockError::ViewBusy { .. })
    ));
    // Nothing moved yet — the view still belongs to w1.
    assert_eq!(ws.owner_of(vid(1)), Some(w1));

    // Phase 2: destination prepared (webview loaded) → commit.
    host.mark_ready(w2);
    let report = ws.commit_move(token).expect("commit");
    assert_eq!(report.from, Some(w1));
    assert_eq!(report.to, w2);
    assert_eq!(ws.owner_of(vid(1)), Some(w2));
    assert_eq!(view_ids(&ws, w1), vec![vid(2)]);
    assert_eq!(view_ids(&ws, w2), vec![vid(3), vid(1)]);

    // Consumer restores state at the destination by logical id — the
    // same id arrived, so the saved state applies unchanged.
    assert_eq!(view_state.get(&report.view).unwrap(), "scroll=42;filter=on");

    // Event order: prepare → tree changes → committed.
    let events = ws.drain_events();
    let committed_pos = events
        .iter()
        .position(|e| matches!(e, DockEvent::MoveCommitted { view, .. } if *view == vid(1)))
        .expect("committed event");
    let prepared_pos = events
        .iter()
        .position(|e| matches!(e, DockEvent::MovePrepared { view } if *view == vid(1)))
        .expect("prepared event");
    assert!(prepared_pos < committed_pos);
}

/// Destination preparation fails (native window creation error /
/// webview init failure / IPC timeout): `abort_move` rolls back —
/// the view never left its source dock.
#[test]
fn window_creation_failure_rolls_back_to_source_dock() {
    let mut host = FakeHost::new();
    let mut ws = DockWorkspace::new();

    let w1 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w1);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();

    // Begin the move toward a new window slot (id reserved by the
    // consumer), then the host fails to create it.
    let token = ws
        .begin_move(vid(1), DockTarget::Window(w1))
        .expect("begin");
    host.fail_creates = 1;
    assert!(host.create_window(&WindowConfig::default()).is_err());

    // Rollback: abort — source untouched, lock cleared.
    ws.abort_move(token).unwrap();
    assert_eq!(ws.owner_of(vid(1)), Some(w1));
    assert_eq!(view_ids(&ws, w1), vec![vid(1)]);
    assert!(!ws.is_transferring(vid(1)));
    assert!(
        ws.drain_events()
            .contains(&DockEvent::MoveAborted { view: vid(1) })
    );
}

/// Commit against a destination that vanished mid-transfer rolls
/// back automatically — `TargetUnavailable`, view stays at source.
#[test]
fn commit_after_destination_destroyed_rolls_back() {
    let mut host = FakeHost::new();
    let mut ws = DockWorkspace::new();

    let w1 = host.create_window(&WindowConfig::default()).unwrap();
    let w2 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();

    let token = ws.begin_move(vid(1), DockTarget::Window(w2)).unwrap();
    host.destroy_window(w2).unwrap();
    ws.remove_window(w2, WindowCloseDisposition::ReleaseViews)
        .unwrap();

    assert!(matches!(
        ws.commit_move(token),
        Err(ruviform_ui::DockError::TargetUnavailable { window }) if window == w2
    ));
    assert_eq!(ws.owner_of(vid(1)), Some(w1));
    assert!(!ws.is_transferring(vid(1)));
}

/// Detach → reattach across windows preserves the logical view id —
/// the identity the consumer's view state is keyed by.
#[test]
fn detach_reattach_preserves_logical_view_id() {
    let mut host = FakeHost::new();
    let mut ws = DockWorkspace::new();

    let w1 = host.create_window(&WindowConfig::default()).unwrap();
    let w2 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(7), DockTarget::Window(w1)).unwrap();

    ws.detach_view(vid(7)).unwrap();
    assert!(ws.is_detached(vid(7)));
    assert_eq!(view_ids(&ws, w1), vec![]);
    assert!(ws.tree(w1).is_none()); // last view gone → tree dropped

    ws.attach_view(vid(7), DockTarget::Window(w2)).unwrap();
    assert_eq!(ws.owner_of(vid(7)), Some(w2));

    // Duplicate attach of the same id is still rejected everywhere.
    assert!(matches!(
        ws.attach_view(vid(7), DockTarget::Window(w1)),
        Err(ruviform_ui::DockError::DuplicateView { .. })
    ));
}

/// A menu-driven move (`move_view`, one-shot) produces exactly the
/// same dock result as a drop-targeted move — the explicit
/// menu-move fallback path needs no drag machinery at all.
#[test]
fn menu_move_fallback_matches_drop_move() {
    let mut ws = DockWorkspace::new();
    ws.register_window(WindowId::new(1).unwrap());
    ws.register_window(WindowId::new(2).unwrap());
    ws.attach_view(vid(1), DockTarget::Window(WindowId::new(1).unwrap()))
        .unwrap();
    ws.attach_view(vid(2), DockTarget::Window(WindowId::new(1).unwrap()))
        .unwrap();

    // "Move to window 2" menu item → one-shot move_view.
    ws.move_view(vid(2), DockTarget::Window(WindowId::new(2).unwrap()))
        .unwrap();
    assert_eq!(ws.owner_of(vid(2)), Some(WindowId::new(2).unwrap()));

    // A zone drop onto the moved view splits its leaf.
    ws.move_view(
        vid(1),
        DockTarget::Zone {
            window: WindowId::new(2).unwrap(),
            anchor: vid(2),
            zone: DropZone::Left,
        },
    )
    .unwrap();
    let root = ws.tree(WindowId::new(2).unwrap()).unwrap().root().clone();
    assert!(matches!(
        root,
        DockNode::Split {
            axis: ruviform_ui::Axis::Horizontal,
            ..
        }
    ));
}

/// Closing a window applies the consumer's explicit disposition —
/// never a silent teardown. `DetachViews` keeps the views (reattach
/// later); `RelocateTo` re-docks them into another window.
#[test]
fn window_close_applies_consumer_chosen_disposition() {
    let mut host = FakeHost::new();
    let mut ws = DockWorkspace::new();

    let w1 = host.create_window(&WindowConfig::default()).unwrap();
    let w2 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(2), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(3), DockTarget::Window(w2)).unwrap();

    // Consumer chooses: detach the views — project stays alive, views
    // parked for reattach.
    let carried = ws
        .remove_window(w1, WindowCloseDisposition::DetachViews)
        .unwrap();
    assert_eq!(carried, vec![vid(1), vid(2)]);
    assert_eq!(ws.detached_views(), vec![vid(1), vid(2)]);
    host.destroy_window(w1).unwrap();

    // Consumer chooses: relocate w2's view into a fresh window.
    let w3 = host.create_window(&WindowConfig::default()).unwrap();
    ws.register_window(w3);
    ws.remove_window(w2, WindowCloseDisposition::RelocateTo { target: w3 })
        .unwrap();
    assert_eq!(ws.owner_of(vid(3)), Some(w3));

    // Reattach the parked views — ids stable end to end.
    ws.attach_view(vid(1), DockTarget::Window(w3)).unwrap();
    assert_eq!(ws.owner_of(vid(1)), Some(w3));
}

/// Layout snapshot round-trip: geometry + monitor signature + dock
/// trees + detached pool survive JSON; a monitor disappearing and a
/// DPI change are corrected at restore, never crash.
#[test]
fn layout_snapshot_roundtrips_with_monitor_correction() {
    let mut ws = DockWorkspace::new();
    let w1 = WindowId::new(1).unwrap();
    let w2 = WindowId::new(2).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(2), DockTarget::Window(w1)).unwrap();
    ws.move_view(
        vid(2),
        DockTarget::Zone {
            window: w1,
            anchor: vid(1),
            zone: DropZone::Right,
        },
    )
    .unwrap();
    ws.attach_view(vid(3), DockTarget::Window(w2)).unwrap();
    ws.add_detached(vid(8)).unwrap();

    // Capture: w2 sat on a second HiDPI monitor.
    let windows = ws.windows();
    let geom = vec![
        WindowGeometry {
            monitor: Some(MonitorHint {
                name: Some("main".to_string()),
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
            }),
            ..geometry(10, 10, 960, 540)
        },
        WindowGeometry {
            monitor: Some(MonitorHint {
                name: Some("external".to_string()),
                width: 2560,
                height: 1440,
                scale_factor: 2.0,
            }),
            ..geometry(2100, 100, 1280, 720)
        },
    ];
    let snap = LayoutSnapshot::capture(&windows, &geom, &ws).unwrap();
    let json = snap.to_json().unwrap();

    // Restart: the external monitor is gone — only `main` remains.
    let current = vec![MonitorInfo {
        name: Some("main".to_string()),
        width: 1920,
        height: 1080,
        x: 0,
        y: 0,
        scale_factor: 1.0,
    }];
    let restored = LayoutSnapshot::from_json(&json)
        .unwrap()
        .restore(&current)
        .unwrap();

    // w1's geometry kept on monitor 0; w2 re-centered onto monitor 0.
    assert_eq!(restored.placements[0].monitor, Some(0));
    assert_eq!(
        (restored.placements[0].x, restored.placements[0].y),
        (10, 10)
    );
    assert_eq!(restored.placements[1].monitor, Some(0));
    let p2 = &restored.placements[1];
    assert!(p2.x >= 0 && p2.y >= 0 && p2.width <= 1920 && p2.height <= 1080);

    // Replay into a fresh workspace over fresh windows.
    let mut host = FakeHost::new();
    let mut ws2 = DockWorkspace::new();
    for placement in &restored.placements {
        let cfg = WindowConfig {
            width: placement.width,
            height: placement.height,
            position: Some((placement.x, placement.y)),
            ..WindowConfig::default()
        };
        let w = host.create_window(&cfg).unwrap();
        ws2.register_window(w);
        if let Some(tree) = placement.tree.clone() {
            ws2.apply_restored_tree(w, tree).unwrap();
        }
        assert_eq!(
            host.created_configs[&w].position,
            Some((placement.x, placement.y))
        );
    }
    for v in restored.detached {
        ws2.add_detached(v).unwrap();
    }
    let new_windows = ws2.windows();
    assert_eq!(ws2.owner_of(vid(1)), Some(new_windows[0]));
    assert_eq!(ws2.owner_of(vid(2)), Some(new_windows[0]));
    assert_eq!(ws2.owner_of(vid(3)), Some(new_windows[1]));
    assert!(ws2.is_detached(vid(8)));
    // The split structure came back.
    assert!(matches!(
        ws2.tree(new_windows[0]).unwrap().root(),
        DockNode::Split { .. }
    ));

    // A corrupt snapshot restores to the safe empty default.
    let fallback = LayoutSnapshot {
        schema: 999,
        ..LayoutSnapshot::default()
    }
    .restore_or_default(&current);
    assert!(fallback.placements.is_empty());
}
