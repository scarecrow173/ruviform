//! Issue #464 consumer-neutral Scenario Contract — GPUI adapter side.
//!
//! Runs the same `ruviform_ui_test::scenario` surface against
//! `GpuiGuiTest` (`GpuiObserver` under the hood, `gpui-test` feature).
//! The parity scenario function is shared with the Fake adapter test —
//! this file proves the same ordinary-Rust scenario executes on both.

use std::collections::BTreeSet;

use ruviform_ui::behavior::{CommandIntent, CommandPhase, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec, Orientation,
    ViewHost, ViewSpec,
};
use ruviform_ui::{Event, WindowSpec};
use ruviform_ui_gpui::{GpuiGuiTest, ViewMount};
use ruviform_ui_test::{
    ScenarioCapability, ScenarioDriver, ScenarioError, SemanticGuiTest, TargetQuery, begin,
    parity_gesture_scenario,
};
use serde_json::json;

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn fader(name: &str) -> ComponentSpec {
    ComponentSpec::Fader(ControlSpec {
        key: key(name),
        control_id: None,
        target: Some(format!("mixer.{name}")),
        descriptor: ruviform_ui::behavior::ValueDescriptor {
            min: 0.0,
            max: 1.0,
            default_value: Some(0.5),
            step: Some(0.05),
            ..ruviform_ui::behavior::ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: Orientation::Vertical,
        detach_policy: ruviform_ui::behavior::DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: ruviform_ui::view::A11ySpec::default(),
    })
}

fn panel(children: Vec<ComponentSpec>) -> ComponentSpec {
    ComponentSpec::Panel(ruviform_ui::view::PanelSpec {
        key: key("panel"),
        title: None,
        children,
        visual: ComponentVisualState::default(),
        a11y: ruviform_ui::view::A11ySpec::default(),
    })
}

fn fader_view() -> ViewSpec {
    ViewSpec {
        root: panel(vec![fader("fader-ch1"), fader("fader-ch2")]),
    }
}

/// Adapter with the shared fader view mounted in one window, settled
/// once so the window opens and the view renders.
fn mounted() -> (GpuiGuiTest, ruviform_ui_test::ScenarioTarget) {
    let mut gui = GpuiGuiTest::new();
    let mut host = ViewHost::new(
        ViewInstanceId::new(464).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(fader_view()).expect("mount");
    host.drain_events();
    let window = gui
        .observer_mut()
        .driver_mut()
        .create_view_window(&WindowSpec::default(), ViewMount { host, theme: None })
        .expect("create_view_window");
    gui.settle().expect("settle mount");
    let target = gui
        .resolve_target(&TargetQuery::new(window, ViewInstanceId::new(464).unwrap()))
        .expect("resolve");
    (gui, target)
}

#[test]
fn gpui_parity_gesture_mints_intents_and_commits_value() {
    let (mut gui, target) = mounted();
    let k = key("fader-ch1");

    let (intents, snapshot) =
        parity_gesture_scenario(&mut gui, &target, &k, "g1", ValueInput::Plain(0.8))
            .expect("parity scenario");

    let kinds: Vec<&'static str> = intents
        .iter()
        .map(|m| match m.intent {
            CommandIntent::Begin { .. } => "begin",
            CommandIntent::Preview { .. } => "preview",
            CommandIntent::Commit { .. } => "commit",
            CommandIntent::Cancel { .. } => "cancel",
            CommandIntent::Action { .. } => "action",
            _ => "other",
        })
        .collect();
    assert!(kinds.contains(&"begin"), "kinds: {kinds:?}");
    assert!(kinds.contains(&"commit"), "kinds: {kinds:?}");
    assert!(snapshot.mounted);
    assert_eq!(
        snapshot.committed.map(|v| v.normalized),
        Some(0.8),
        "commit promotes the previewed value"
    );
}

#[test]
fn gpui_stale_generation_and_unknown_component_are_explicit() {
    let (mut gui, target) = mounted();
    let k = key("fader-ch1");

    // Detach bumps the host generation → pinned target goes stale.
    gui.detach_view(target.window, "test").expect("detach");

    let err = gui
        .dispatch_input(&target, &k, begin("g-stale"))
        .expect_err("stale generation must fail");
    assert!(
        matches!(err, ScenarioError::StaleGeneration { .. }),
        "expected StaleGeneration, got {err:?}"
    );

    let target = gui
        .resolve_target(&TargetQuery::new(target.window, target.view))
        .expect("re-resolve");
    let err = gui
        .observe(&target, &key("nope"))
        .expect_err("unmounted component must fail");
    assert!(matches!(err, ScenarioError::UnknownComponent { .. }));

    let err = gui
        .resolve_target(&TargetQuery::new(
            target.window,
            ViewInstanceId::new(999).unwrap(),
        ))
        .expect_err("missing view must fail");
    assert!(matches!(err, ScenarioError::NoSuchView { .. }));
}

#[test]
fn gpui_capability_preflight_matches_fake() {
    let (gui, _target) = mounted();
    assert_eq!(gui.driver(), ScenarioDriver::GpuiSimulated);
    assert_eq!(
        gui.capabilities(),
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    );
    let err = gui
        .require_capabilities(&BTreeSet::from([ScenarioCapability::VisualCapture]))
        .expect_err("visual capture is not a simulated capability");
    assert!(matches!(err, ScenarioError::Unsupported { .. }));
}

#[test]
fn gpui_post_event_routes_receipt_and_projection() {
    let (mut gui, target) = mounted();
    let k = key("fader-ch1");

    let (intents, _snapshot) =
        parity_gesture_scenario(&mut gui, &target, &k, "g2", ValueInput::Plain(0.7))
            .expect("parity scenario");
    let commit = intents
        .iter()
        .find(|m| matches!(m.intent, CommandIntent::Commit { .. }))
        .expect("commit intent");
    let control_id = match &commit.intent {
        CommandIntent::Commit { control_id, .. } => control_id.clone(),
        _ => unreachable!(),
    };

    gui.post_event(
        &target,
        &Event::result_ok(
            commit.command_id.clone(),
            json!({"status": "applied", "revision": 1}),
        ),
    )
    .expect("post receipt");
    gui.settle().expect("settle receipt");
    let snapshot = gui.observe(&target, &k).expect("observe");
    assert_eq!(snapshot.phase, CommandPhase::Applied);

    gui.post_event(
        &target,
        &Event::Notify {
            seq: 1,
            data: json!({
                "control_id": control_id,
                "revision": 2,
                "value": {"normalized": 0.25},
            }),
        },
    )
    .expect("post projection");
    gui.settle().expect("settle projection");
    let snapshot = gui.observe(&target, &k).expect("observe");
    assert!(snapshot.observed.is_some(), "projection observed");
    assert_eq!(snapshot.phase, CommandPhase::Observed);
}
