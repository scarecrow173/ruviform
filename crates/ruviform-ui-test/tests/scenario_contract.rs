//! Issue #464 consumer-neutral Scenario Contract — Fake adapter side.
//!
//! Exercises `ruviform_ui_test::scenario` against `FakeGuiTest`: the
//! same `SemanticGuiTest` surface the GPUI adapter implements under
//! `gpui-test`. Covers target resolution, stale epoch/generation
//! errors, capability pre-flight, intent drain, and receipt/projection
//! delivery through `ViewHost::deliver_event`.

use std::collections::BTreeSet;

use ruviform_ui::behavior::{CommandIntent, CommandPhase, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec, Orientation,
    ViewSpec,
};
use ruviform_ui::{Event, WindowId};
use ruviform_ui_test::{
    FakeGuiTest, RunId, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioId,
    ScenarioReport, ScenarioTarget, ScenarioVerdict, SemanticGuiTest, TargetQuery, begin, cancel,
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

/// Mounted adapter + resolved target for the shared fader view.
fn mounted() -> (FakeGuiTest, WindowId, ViewInstanceId, ScenarioTarget) {
    let mut gui = FakeGuiTest::new();
    let window = gui.open_window();
    let view = ViewInstanceId::new(464).expect("nonzero");
    gui.mount_view(window, view, fader_view()).expect("mount");
    let target = gui
        .resolve_target(&TargetQuery::new(window, view))
        .expect("resolve");
    (gui, window, view, target)
}

#[test]
fn fake_parity_gesture_mints_intents_and_commits_value() {
    let (mut gui, _window, _view, target) = mounted();
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
fn stale_generation_and_epoch_are_explicit_errors() {
    let (mut gui, window, view, target) = mounted();
    let k = key("fader-ch1");

    // Generation bump: detach advances generation → pinned target stale.
    gui.detach_view(window, view, "test").expect("detach");
    let err = gui
        .dispatch_input(&target, &k, begin("g-stale"))
        .expect_err("stale generation must fail");
    assert!(
        matches!(err, ScenarioError::StaleGeneration { .. }),
        "expected StaleGeneration, got {err:?}"
    );

    // Epoch bump: remounting the same view id mints a new attach epoch.
    gui.mount_view(window, view, fader_view()).expect("remount");
    let err = gui.observe(&target, &k).expect_err("stale epoch must fail");
    assert!(
        matches!(err, ScenarioError::StaleEpoch { .. }),
        "expected StaleEpoch, got {err:?}"
    );
}

#[test]
fn resolve_target_honours_query_pins() {
    let (mut gui, window, view, target) = mounted();

    let err = gui
        .resolve_target(
            &TargetQuery::new(window, view)
                .attach_epoch(AttachEpoch::new(999))
                .generation(target.generation),
        )
        .expect_err("wrong epoch pin must fail");
    assert!(matches!(err, ScenarioError::StaleEpoch { .. }));

    let err = gui
        .resolve_target(&TargetQuery::new(window, view).generation(target.generation + 1))
        .expect_err("wrong generation pin must fail");
    assert!(matches!(err, ScenarioError::StaleGeneration { .. }));
}

#[test]
fn unknown_window_view_and_component_are_explicit() {
    let (mut gui, window, view, _target) = mounted();

    let err = gui
        .resolve_target(&TargetQuery::new(window, ViewInstanceId::new(999).unwrap()))
        .expect_err("missing view must fail");
    assert!(matches!(err, ScenarioError::NoSuchView { .. }));

    let err = gui
        .resolve_target(&TargetQuery::new(WindowId::new(999).unwrap(), view))
        .expect_err("missing window must fail");
    assert!(matches!(err, ScenarioError::NoSuchWindow { .. }));

    let target = gui
        .resolve_target(&TargetQuery::new(window, view).component(key("nope")))
        .expect_err("unmounted component must fail");
    assert!(matches!(target, ScenarioError::UnknownComponent { .. }));

    let target = gui
        .resolve_target(&TargetQuery::new(window, view))
        .expect("resolve");
    let err = gui
        .observe(&target, &key("nope"))
        .expect_err("observe of unmounted key must fail");
    assert!(matches!(err, ScenarioError::UnknownComponent { .. }));
}

#[test]
fn capability_preflight_rejects_unsupported_levels() {
    let (gui, _window, _view, _target) = mounted();

    assert_eq!(
        gui.capabilities(),
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    );
    assert_eq!(gui.driver(), ScenarioDriver::Fake);

    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))
        .expect("semantic action is supported");

    let err = gui
        .require_capabilities(&BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::SimulatedPointerOrKeyboard,
        ]))
        .expect_err("simulated pointer is not a fake capability");
    assert!(matches!(
        err,
        ScenarioError::Unsupported {
            capability: ScenarioCapability::SimulatedPointerOrKeyboard
        }
    ));
}

#[test]
fn post_event_routes_receipt_and_projection_to_binding() {
    let (mut gui, _window, _view, target) = mounted();
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

    // Receipt for the minted command id settles the binding phase.
    gui.post_event(
        &target,
        &Event::result_ok(
            commit.command_id.clone(),
            json!({"status": "applied", "revision": 1}),
        ),
    )
    .expect("post receipt");
    gui.settle().expect("settle");
    let snapshot = gui.observe(&target, &k).expect("observe");
    assert_eq!(snapshot.phase, CommandPhase::Applied);

    // Authoritative projection lands through the same deliver_event path.
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
    gui.settle().expect("settle");
    let snapshot = gui.observe(&target, &k).expect("observe");
    assert!(snapshot.observed.is_some(), "projection observed");
    assert_eq!(snapshot.phase, CommandPhase::Observed);
}

#[test]
fn cancel_and_set_helpers_are_plain_control_inputs() {
    let (mut gui, _window, _view, target) = mounted();
    let k = key("fader-ch1");

    gui.dispatch_input(&target, &k, begin("g3")).expect("begin");
    gui.dispatch_input(&target, &k, cancel("g3"))
        .expect("cancel");
    gui.settle().expect("settle");
    let snapshot = gui.observe(&target, &k).expect("observe");
    assert_eq!(snapshot.phase, CommandPhase::Idle);

    // Report skeleton keeps the evidence level honest.
    let report = ScenarioReport::new(
        RunId::new("run-1"),
        ScenarioId::new("scenario-parity"),
        None,
        gui.driver(),
    );
    assert_eq!(report.verdict, ScenarioVerdict::NotRun);
    assert_eq!(report.driver, ScenarioDriver::Fake);
}

#[test]
fn multi_window_targets_resolve_independently() {
    let mut gui = FakeGuiTest::new();
    let w1 = gui.open_window();
    let w2 = gui.open_window();
    let v1 = ViewInstanceId::new(1).unwrap();
    let v2 = ViewInstanceId::new(2).unwrap();
    gui.mount_view(w1, v1, fader_view()).expect("mount w1");
    gui.mount_view(w2, v2, fader_view()).expect("mount w2");

    let t1 = gui
        .resolve_target(&TargetQuery::new(w1, v1))
        .expect("resolve w1");
    let t2 = gui
        .resolve_target(&TargetQuery::new(w2, v2))
        .expect("resolve w2");
    assert_ne!(t1.attach_epoch, t2.attach_epoch);

    gui.dispatch_input(&t2, &key("fader-ch1"), begin("g4"))
        .expect("dispatch w2");
    gui.settle().expect("settle");
    // Every minted intent belongs to w2's view namespace — w1 minted none.
    let intents = gui.drain_intents();
    assert!(!intents.is_empty(), "w2 gesture minted intents");
    assert!(
        intents.iter().all(|m| m.command_id.contains("view-2:")),
        "intents: {:?}",
        intents.iter().map(|m| &m.command_id).collect::<Vec<_>>()
    );
}
