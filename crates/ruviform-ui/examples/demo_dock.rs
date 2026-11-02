//! `demo_dock` — headless narrated run of the Issue #182 dock
//! foundation: windows, tab leaves, splits, the two-phase cross-window
//! move (destination prepared first, ownership switched second),
//! failure rollback, detach/reattach preserving the logical view id,
//! consumer-chosen window-close dispositions, and a layout snapshot
//! save/restore round-trip with monitor correction.
//!
//! Everything runs against [`DockWorkspace`] — pure model, no GUI —
//! so the same flow a webview front-end would drive is exercised
//! entirely headless.
//!
//! Usage: `demo_dock` (no arguments; exit 0 on success).

use ruviform_ui::{
    DockEvent, DockNode, DockPath, DockTarget, DockWorkspace, DropZone, LayoutSnapshot,
    MonitorHint, MonitorInfo, ViewInstanceId, WindowCloseDisposition, WindowGeometry, WindowId,
};

fn vid(raw: u64) -> ViewInstanceId {
    ViewInstanceId::new(raw).expect("nonzero")
}

fn wid(raw: u64) -> WindowId {
    WindowId::new(raw).expect("nonzero")
}

fn describe(ws: &DockWorkspace, window: WindowId) -> String {
    fn node(n: &DockNode) -> String {
        match n {
            DockNode::Tabs { views, active } => {
                let list: Vec<String> = views
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        if i == *active {
                            format!("[{v}]")
                        } else {
                            v.to_string()
                        }
                    })
                    .collect();
                format!("Tabs({})", list.join(" "))
            }
            DockNode::Split {
                axis,
                ratio,
                children,
            } => {
                let parts: Vec<String> = children.iter().map(node).collect();
                format!("Split({axis:?},{ratio:.2} | {})", parts.join(" "))
            }
            _ => "?".to_string(),
        }
    }
    match ws.tree(window) {
        Some(t) => format!("{window} = {}", node(t.root())),
        None => format!("{window} = (empty)"),
    }
}

fn drain(ws: &mut DockWorkspace) {
    for ev in ws.drain_events() {
        match ev {
            DockEvent::TreeChanged { window } => println!("    event: TreeChanged({window})"),
            DockEvent::MovePrepared { view } => println!("    event: MovePrepared({view})"),
            DockEvent::MoveCommitted { view, from, to } => {
                println!("    event: MoveCommitted({view}: {from:?} -> {to})");
            }
            DockEvent::MoveAborted { view } => println!("    event: MoveAborted({view})"),
            DockEvent::WindowRemoved { window, views } => {
                println!("    event: WindowRemoved({window}, {views:?})");
            }
            _ => println!("    event: {ev:?}"),
        }
    }
}

fn dock_and_split(ws: &mut DockWorkspace, w1: WindowId, w2: WindowId) {
    println!("== dock ==  two windows open, three views docked");
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(vid(1), DockTarget::Window(w1)).unwrap();
    ws.attach_view(vid(2), DockTarget::Window(w1)).unwrap(); // tab onto v1
    ws.attach_view(vid(3), DockTarget::Window(w2)).unwrap();
    println!("  {}", describe(ws, w1));
    println!("  {}", describe(ws, w2));
    drain(ws);

    println!("== split ==  v2 dropped right of v1 → 35/65 split");
    ws.move_view(
        vid(2),
        DockTarget::Zone {
            window: w1,
            anchor: vid(1),
            zone: DropZone::Right,
        },
    )
    .unwrap();
    ws.set_ratio(w1, &DockPath::root(), 0.35).unwrap();
    println!("  {}", describe(ws, w1));
    drain(ws);
}

fn move_and_rollback(ws: &mut DockWorkspace, w1: WindowId, w2: WindowId) {
    println!("== move ==  v1 begins a two-phase move to w2");
    let token = ws.begin_move(vid(1), DockTarget::Window(w2)).unwrap();
    println!("  in transfer: edits on v1 are now rejected (ViewBusy)");
    println!(
        "  activate while locked → {:?}",
        ws.activate_view(vid(1)).unwrap_err()
    );
    println!("  ...destination webview ready → commit");
    let report = ws.commit_move(token).unwrap();
    println!(
        "  committed: v{} from {:?} to {}",
        report.view.get(),
        report.from,
        report.to
    );
    println!("  {}", describe(ws, w1));
    println!("  {}", describe(ws, w2));
    drain(ws);

    println!("== rollback ==  v3's destination disappears mid-transfer");
    let token = ws.begin_move(vid(3), DockTarget::Window(w1)).unwrap();
    // The destination window is destroyed before commit — the move
    // rolls back to the source dock automatically.
    ws.remove_window(w1, WindowCloseDisposition::ReleaseViews)
        .unwrap();
    match ws.commit_move(token) {
        Err(e) => println!("  commit failed as designed: {e}"),
        Ok(_) => println!("  unexpected success"),
    }
    println!("  v3 stayed put → owner {:?}", ws.owner_of(vid(3)));
    drain(ws);
}

fn detach_and_close(ws: &mut DockWorkspace, w1: WindowId) {
    println!("== detach ==  v3 parked, then reattached — same logical id");
    ws.detach_view(vid(3)).unwrap();
    println!("  detached pool: {:?}", ws.detached_views());
    ws.register_window(w1);
    ws.attach_view(vid(3), DockTarget::Window(w1)).unwrap();
    println!("  reattached to w1 → owner {:?}", ws.owner_of(vid(3)));
    drain(ws);

    println!("== close ==  w1 closes; consumer chooses DetachViews");
    let carried = ws
        .remove_window(w1, WindowCloseDisposition::DetachViews)
        .unwrap();
    println!("  carried views {carried:?} → parked, not destroyed");
    println!("  detached pool: {:?}", ws.detached_views());
    drain(ws);
}

fn snapshot_restore(ws: &mut DockWorkspace, w1: WindowId) {
    println!("== restore ==  snapshot → JSON → restore with a monitor gone");
    ws.register_window(w1);
    ws.attach_view(vid(3), DockTarget::Window(w1)).unwrap();
    let windows = ws.windows();
    let geometry: Vec<WindowGeometry> = windows
        .iter()
        .map(|_| WindowGeometry {
            x: 2600,
            y: 80,
            width: 1280,
            height: 720,
            monitor: Some(MonitorHint {
                name: Some("external".to_string()),
                width: 2560,
                height: 1440,
                scale_factor: 2.0,
            }),
        })
        .collect();
    let snap = LayoutSnapshot::capture(&windows, &geometry, ws).unwrap();
    let json = snap.to_json().unwrap();
    println!("  saved {} bytes of layout JSON", json.len());

    // The "external" monitor is gone — only a 1920x1080 panel remains.
    let monitors = vec![MonitorInfo {
        name: Some("laptop".to_string()),
        width: 1920,
        height: 1080,
        x: 0,
        y: 0,
        scale_factor: 1.25,
    }];
    let restored = LayoutSnapshot::from_json(&json)
        .unwrap()
        .restore(&monitors)
        .unwrap();
    for (i, p) in restored.placements.iter().enumerate() {
        println!(
            "  window[{i}] → {}x{} @ ({},{}) monitor {:?}",
            p.width, p.height, p.x, p.y, p.monitor
        );
    }
    println!("  all windows re-centered on the surviving monitor");
}

fn main() {
    let mut ws = DockWorkspace::new();
    let (w1, w2) = (wid(1), wid(2));
    dock_and_split(&mut ws, w1, w2);
    move_and_rollback(&mut ws, w1, w2);
    detach_and_close(&mut ws, w1);
    snapshot_restore(&mut ws, w1);
    println!("demo_dock: done");
}
