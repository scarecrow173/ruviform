//! Driver-neutral Window/Dock lifecycle fixture (Issue #386).
//!
//! One scenario runs against every driver: [`run_lifecycle`] drives a
//! [`LifecycleHarness`] through two windows, a 2-phase Dock move with
//! rollback, detach/reattach, veto/defer/approve closes, the unanswered
//! close default, late `post_event` receipt, id non-reuse and
//! tear-down. It returns the normalized window lifecycle as a
//! [`Trace`], which must equal [`expected_trace`] on each driver. The
//! scenario speaks only the neutral `ruviform-ui` contract; the
//! driver-specific event pumping lives in the harness.
//!
//! Out of scope here (`NotRun` — real OS only): DPI/scale transitions,
//! IME, OS-driven focus, GPU device loss. Their events are ignored by
//! the trace because their timing is driver-specific.
//!
//! **RT contract:** non-RT only. Test-only crate.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use ruviform_ui::dock::{
    DockNode, DockTarget, DockTree, DockWorkspace, ViewInstanceId, WindowCloseDisposition,
};
use ruviform_ui::{
    CloseDecision, DriverEvent, Event, LogicalSize, UiError, WindowDriver, WindowId,
    WindowLifecycle, WindowSpec,
};
use serde_json::json;

/// Normalized lifecycle per named window, in delivery order.
pub type Trace = BTreeMap<&'static str, Vec<&'static str>>;

/// Event handler a harness turn delivers driver events to.
pub type Handler = Box<dyn FnMut(DriverEvent, &mut dyn WindowDriver)>;

/// The driver under test, seen through the neutral port.
pub trait LifecycleHarness {
    /// The driver as the neutral [`WindowDriver`] port.
    fn driver(&mut self) -> &mut dyn WindowDriver;

    /// One driver loop turn: applies queued backend work and delivers
    /// the resulting events to `on_event`. The handler may call back
    /// into the driver (for example `respond_close`).
    fn settle(&mut self, on_event: Handler);

    /// Whether `exit` was requested.
    fn exited(&self) -> bool;

    /// Events delivered to `window`'s view so far.
    fn received(&self, window: WindowId) -> usize;
}

/// The normalized trace every driver must produce.
#[must_use]
pub fn expected_trace() -> Trace {
    let closed = ["Created", "Ready", "CloseRequested", "Destroyed"];
    BTreeMap::from([
        ("main", closed.to_vec()),
        (
            "inspector",
            vec![
                "Created",
                "Ready",
                "CloseRequested",
                "CloseRequested",
                "Destroyed",
            ],
        ),
        ("late", closed.to_vec()),
        ("reborn", closed.to_vec()),
    ])
}

/// Runs the shared lifecycle scenario and returns its normalized trace.
///
/// # Panics
/// Panics when the driver or the Dock breaks the shared contract —
/// the fixture is an assertion suite.
pub fn run_lifecycle<H: LifecycleHarness>(harness: &mut H) -> Trace {
    let mut run = Run {
        h: harness,
        ws: DockWorkspace::new(),
        names: BTreeMap::new(),
        answer: Rc::new(Cell::new(None)),
        log: Rc::new(RefCell::new(Vec::new())),
    };

    // Two windows, each made visible by the driver's own turn.
    let main = run.create("main");
    let inspector = run.create("inspector");
    run.settle();
    run.assert_live(&[main, inspector]);

    // Dock: both views start in `main`. v2 moves to `inspector` via a
    // committed 2-phase move; v1's move is aborted and must stay put.
    let (v1, v2) = (view(1), view(2));
    run.ws
        .attach_tree(main, tree(&[v1, v2]))
        .expect("attach tree");
    let token = run
        .ws
        .begin_move(v2, DockTarget::Window(inspector))
        .expect("begin move");
    run.ws.commit_move(token).expect("commit move");
    assert_eq!(run.ws.owner_of(v2), Some(inspector));
    let token = run
        .ws
        .begin_move(v1, DockTarget::Window(inspector))
        .expect("begin rollback move");
    run.ws.abort_move(token).expect("abort move");
    assert_eq!(run.ws.owner_of(v1), Some(main));

    // Detach then reattach keeps the ViewInstanceId.
    run.ws.detach_view(v1).expect("detach view");
    assert!(run.ws.is_detached(v1));
    run.ws
        .attach_view(v1, DockTarget::Window(inspector))
        .expect("reattach view");
    assert_eq!(run.ws.owner_of(v1), Some(inspector));

    // Window ops every driver accepts. Releasing focus is a capability
    // the driver may report Unsupported, but never as a host error.
    run.driver()
        .resize(main, LogicalSize::new(1024, 768))
        .expect("resize");
    run.driver().focus(main, true).expect("focus");
    match run.driver().focus(main, false) {
        Ok(()) | Err(UiError::Unsupported { .. }) => {}
        Err(other) => panic!("focus release must be Ok or Unsupported, got {other:?}"),
    }

    // Late receipt: a message posted before `Ready` is held and
    // delivered once the window becomes ready.
    let late = run.create("late");
    run.driver()
        .post_event(late, &Event::result_ok("late-1", json!({})));
    run.settle();
    assert_eq!(run.h.received(late), 1);

    // An unanswered close defaults to Approve for that window only.
    run.close(late, None);
    run.assert_live(&[main, inspector]);

    // Veto keeps `inspector` open. Defer keeps it close-pending until
    // the consumer approves it through `respond_close`.
    run.close(inspector, Some(CloseDecision::Veto));
    run.assert_live(&[main, inspector]);
    run.close(inspector, Some(CloseDecision::Defer));
    run.assert_live(&[main, inspector]);
    run.driver()
        .respond_close(inspector, CloseDecision::Approve)
        .expect("approve deferred close");
    run.settle();
    run.assert_live(&[main]);

    // The consumer disposes of the closed window's views explicitly.
    let moved = run
        .ws
        .remove_window(
            inspector,
            WindowCloseDisposition::RelocateTo { target: main },
        )
        .expect("relocate views");
    assert_eq!(moved.len(), 2);
    assert_eq!(run.ws.owner_of(v1), Some(main));
    assert_eq!(run.ws.owner_of(v2), Some(main));

    // Ids are never reused after destroy.
    let reborn = run.create("reborn");
    assert!(![late, inspector].contains(&reborn));
    run.settle();

    // Tear-down: close the remaining windows, drop the dock, exit.
    run.close(reborn, None);
    run.close(main, None);
    run.assert_live(&[]);
    run.ws
        .remove_window(main, WindowCloseDisposition::DetachViews)
        .expect("detach views on teardown");
    assert_eq!(run.ws.detached_views().len(), 2);
    run.driver().exit();
    run.settle();
    assert!(run.h.exited());

    run.trace()
}

/// Lifecycle kinds the trace records. Geometry, focus, scale and
/// action events are driver-timing dependent and stay out of it.
fn kind(event: &WindowLifecycle) -> Option<&'static str> {
    Some(match event {
        WindowLifecycle::Created => "Created",
        WindowLifecycle::Ready => "Ready",
        WindowLifecycle::CloseRequested => "CloseRequested",
        WindowLifecycle::Destroyed => "Destroyed",
        _ => return None,
    })
}

fn view(raw: u64) -> ViewInstanceId {
    ViewInstanceId::new(raw).expect("nonzero view id")
}

fn tree(views: &[ViewInstanceId]) -> DockTree {
    DockTree::new(DockNode::tabs(views.to_vec(), 0).expect("tabs")).expect("tree")
}

struct Run<'a, H: LifecycleHarness> {
    h: &'a mut H,
    ws: DockWorkspace,
    names: BTreeMap<WindowId, &'static str>,
    answer: Rc<Cell<Option<CloseDecision>>>,
    log: Rc<RefCell<Vec<(WindowId, &'static str)>>>,
}

impl<H: LifecycleHarness> Run<'_, H> {
    fn driver(&mut self) -> &mut dyn WindowDriver {
        self.h.driver()
    }

    fn create(&mut self, name: &'static str) -> WindowId {
        let spec = WindowSpec {
            title: name.to_string(),
            ..WindowSpec::default()
        };
        let id = self.driver().create_window(&spec).expect("create window");
        self.names.insert(id, name);
        self.ws.register_window(id);
        id
    }

    /// One driver turn. Records lifecycle kinds and answers the next
    /// `CloseRequested` with the pending `answer`; an unanswered close
    /// keeps the driver's default.
    fn settle(&mut self) {
        let (answer, log) = (self.answer.clone(), self.log.clone());
        self.h.settle(Box::new(move |ev, driver| {
            let DriverEvent::Windowed { window, event } = ev else {
                return;
            };
            if let Some(name) = kind(&event) {
                log.borrow_mut().push((window, name));
            }
            if matches!(event, WindowLifecycle::CloseRequested)
                && let Some(decision) = answer.take()
            {
                driver
                    .respond_close(window, decision)
                    .expect("answer pending close");
            }
        }));
    }

    /// Live windows as a set — the contract gives `windows()` no order.
    fn assert_live(&mut self, expected: &[WindowId]) {
        let mut live = self.driver().windows();
        live.sort_unstable();
        let mut expected = expected.to_vec();
        expected.sort_unstable();
        assert_eq!(live, expected);
    }

    fn close(&mut self, window: WindowId, answer: Option<CloseDecision>) {
        self.answer.set(answer);
        self.driver().request_close(window);
        self.settle();
    }

    fn trace(&self) -> Trace {
        let mut trace = Trace::new();
        for (window, name) in self.log.borrow().iter() {
            trace.entry(self.names[window]).or_default().push(*name);
        }
        trace
    }
}
