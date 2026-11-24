//! Issue #944 contract test: `rvf.ui.command.v1` user-entered params.
//!
//! The typed profile inside `Action.params` (`CommandParam` /
//! `CommandParams`), the `ViewHost::dispatch_action` mint path
//! (widget→host), the `Notify`/`StateProjection` path carrying params
//! payloads back (host→widget), and the params-less non-regression —
//! all on the headless fake layer. Design record:
//! `docs/designs/issue-944-ui-command-params.md`.

use ruviform_ui::behavior::{
    CommandIntent, CommandParam, CommandParams, CommandPhase, ReceiptStatus, is_droppable,
};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::{Event, Intent, parse_intent};
use ruviform_ui::view::{
    A11ySpec, AttachEpoch, ButtonSpec, ComponentKey, ComponentSpec, ComponentVisualState,
    LabelSpec, PanelSpec, ViewError, ViewEvent, ViewHost, ViewSpec,
};
use serde_json::json;

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn button(name: &str) -> ComponentSpec {
    ComponentSpec::Button(ButtonSpec {
        key: key(name),
        label: name.to_string(),
        target: "view".to_string(),
        action: "confirm".to_string(),
        params: None,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

fn label(name: &str) -> ComponentSpec {
    ComponentSpec::Label(LabelSpec {
        key: key(name),
        text: name.to_string(),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

fn host_with(children: Vec<ComponentSpec>) -> ViewHost {
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: None,
            children,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    })
    .expect("mount");
    host.drain_events();
    host
}

/// One param of each basic type the profile carries — the sample a
/// user-entered widget produces at runtime.
fn sample_params() -> CommandParams {
    let mut p = CommandParams::new();
    p.insert("ppqn", CommandParam::Integer(960));
    p.insert("ratio", CommandParam::Number(0.5));
    p.insert("label", CommandParam::Text("verse-1".to_string()));
    p.insert("mode", CommandParam::Enum("snap".to_string()));
    p.insert("apply", CommandParam::Bool(true));
    p.insert("at", CommandParam::Point { x: 0.25, y: 0.75 });
    p
}

/// `sample_params` as it decodes off the wire: `Enum` is wire-identical
/// to `Text` (a JSON string), so canonical decode maps it back to
/// `Text` — the documented asymmetry.
fn decoded_params() -> CommandParams {
    let mut p = sample_params();
    *p.0.get_mut("mode").expect("mode") = CommandParam::Text("snap".to_string());
    p
}

fn minted_intent(host: &mut ViewHost) -> (CommandIntent, String, bool) {
    let events = host.drain_events();
    let [event] = events.as_slice() else {
        panic!("expected exactly one event, got {events:?}");
    };
    match event {
        ViewEvent::Intent {
            intent,
            command_id,
            expect_receipt,
        } => (intent.clone(), command_id.clone(), *expect_receipt),
        other => panic!("expected Intent event, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Canonical codec — the typed profile of `Action.params`
// ---------------------------------------------------------------------

#[test]
fn command_param_canonical_codec() {
    let cases: Vec<(CommandParam, serde_json::Value)> = vec![
        (CommandParam::Integer(960), json!(960)),
        (CommandParam::Integer(-3), json!(-3)),
        (CommandParam::Number(0.5), json!(0.5)),
        (CommandParam::Text("verse-1".to_string()), json!("verse-1")),
        (CommandParam::Enum("snap".to_string()), json!("snap")),
        (CommandParam::Bool(true), json!(true)),
        (
            CommandParam::Point { x: 0.25, y: 0.75 },
            json!({ "x": 0.25, "y": 0.75 }),
        ),
    ];
    for (param, wire) in cases {
        assert_eq!(param.to_value().as_ref(), Some(&wire), "{param:?}");
        // An enum name decodes as text — membership is the consumer's
        // action-schema concern, not a wire-level distinction.
        let expect = match param {
            CommandParam::Enum(s) => CommandParam::Text(s),
            other => other,
        };
        assert_eq!(CommandParam::from_value(&wire), Some(expect));
    }

    // Non-canonical shapes are rejected, never defaulted into a value.
    for bad in [
        json!(null),
        json!([1, 2]),
        json!({ "a": 1 }),                       // object that is not a point
        json!({ "x": 1.0, "y": 2.0, "z": 3.0 }), // extra key
        json!({ "x": "a", "y": "b" }),           // non-numeric members
    ] {
        assert_eq!(CommandParam::from_value(&bad), None, "{bad}");
    }

    // Non-finite values have no wire representation — encode fails
    // rather than emitting `null` or a dropped key.
    assert_eq!(CommandParam::Number(f64::NAN).to_value(), None);
    assert_eq!(CommandParam::Number(f64::INFINITY).to_value(), None);
    assert_eq!(
        CommandParam::Point {
            x: 0.0,
            y: f64::NAN
        }
        .to_value(),
        None
    );
}

#[test]
fn command_params_map_codec() {
    let wire = sample_params().to_value().expect("finite params");
    assert_eq!(
        wire,
        json!({
            "apply": true,
            "at": { "x": 0.25, "y": 0.75 },
            "label": "verse-1",
            "mode": "snap",
            "ppqn": 960,
            "ratio": 0.5,
        })
    );
    assert_eq!(CommandParams::from_value(&wire), Some(decoded_params()));

    // Non-objects and non-canonical members are rejected wholesale.
    assert_eq!(CommandParams::from_value(&json!("not-an-object")), None);
    assert_eq!(
        CommandParams::from_value(&json!({ "ok": 1, "bad": [1] })),
        None
    );

    // A non-finite member poisons the whole payload — no partial emit.
    let mut bad = CommandParams::new();
    bad.insert("ok", CommandParam::Integer(1));
    bad.insert("nan", CommandParam::Number(f64::NAN));
    assert_eq!(bad.to_value(), None);
}

// ---------------------------------------------------------------------
// widget→host — dispatch_action mints a params-carrying Action intent
// ---------------------------------------------------------------------

#[test]
fn dispatch_action_mints_params_intent() {
    let mut host = host_with(vec![button("confirm"), label("status")]);
    let generation = host.generation();
    let command_id = host
        .dispatch_action(
            generation,
            &key("confirm"),
            "view".to_string(),
            "confirm",
            Some(sample_params()),
        )
        .expect("dispatch_action");

    let (intent, minted, expect_receipt) = minted_intent(&mut host);
    assert_eq!(minted, command_id);
    assert!(expect_receipt, "Action expects a receipt");
    assert!(!is_droppable(&intent), "Action is non-droppable");

    // The wire body: a discrete `action` intent on the component key
    // carrying the canonical params object.
    let wire = serde_json::to_value(&intent).expect("serialize intent");
    assert_eq!(
        wire,
        json!({
            "kind": "action",
            "target": "view",
            "control_id": "confirm",
            "sequence": 1,
            "origin_view": "view-1",
            "action": "confirm",
            "params": sample_params().to_value().expect("params"),
        })
    );
    assert_eq!(
        CommandParams::from_value(&wire["params"]),
        Some(decoded_params())
    );

    // The command awaits its receipt on the component's binding.
    assert_eq!(
        host.observe(&key("confirm")).expect("snapshot").phase,
        CommandPhase::Pending
    );
}

/// The same wrap `RvfView::flush_events` applies on the real seam:
/// `{command_id, expect_receipt, intent}` inside a versioned `Intent`
/// envelope on the `rvf.ui.command.v1` op. params must survive the
/// full envelope round trip intact.
#[test]
fn params_survive_rvf_ipc_envelope() {
    let mut host = host_with(vec![button("confirm")]);
    let generation = host.generation();
    let command_id = host
        .dispatch_action(
            generation,
            &key("confirm"),
            "view".to_string(),
            "confirm",
            Some(sample_params()),
        )
        .expect("dispatch_action");
    let (intent, minted, expect_receipt) = minted_intent(&mut host);
    assert_eq!(minted, command_id);

    let envelope = Intent::new(
        command_id.clone(),
        "rvf.ui.command.v1",
        json!({
            "command_id": command_id,
            "expect_receipt": expect_receipt,
            "intent": serde_json::to_value(&intent).expect("serialize intent"),
        }),
    );
    let body = serde_json::to_string(&envelope).expect("envelope json");
    let parsed = parse_intent(&body).expect("envelope decodes");
    assert_eq!(parsed.v, 1);
    assert_eq!(parsed.op, "rvf.ui.command.v1");
    assert_eq!(parsed.id, command_id);
    assert_eq!(
        CommandParams::from_value(&parsed.params["intent"]["params"]),
        Some(decoded_params())
    );
}

// ---------------------------------------------------------------------
// host→widget — receipts and params-shaped projections route back
// ---------------------------------------------------------------------

#[test]
fn params_roundtrip_host_to_widget() {
    let mut host = host_with(vec![button("confirm")]);
    let generation = host.generation();
    let command_id = host
        .dispatch_action(
            generation,
            &key("confirm"),
            "view".to_string(),
            "confirm",
            Some(sample_params()),
        )
        .expect("dispatch_action");
    minted_intent(&mut host);
    assert_eq!(
        host.observe(&key("confirm")).expect("snapshot").phase,
        CommandPhase::Pending
    );

    // The consumer answers the command (applied receipt)...
    assert!(host.deliver_event(&Event::result_ok(
        command_id,
        json!({ "status": "applied", "revision": 1 }),
    )));

    // ...then pushes the authoritative projection carrying a
    // params-shaped payload back to the widget's binding.
    let projected = sample_params().to_value().expect("params");
    assert!(host.deliver_event(&Event::Notify {
        seq: 1,
        data: json!({
            "control_id": "confirm",
            "revision": 1,
            "value": projected,
        }),
    }));

    let snap = host.observe(&key("confirm")).expect("snapshot");
    let observed = snap.observed.expect("observed projection");
    assert_eq!(observed.value, projected);
    assert_eq!(
        CommandParams::from_value(&observed.value),
        Some(decoded_params())
    );
    // A landed projection retires the applied head — the observed
    // truth is what the display reports.
    assert_eq!(snap.phase, CommandPhase::Observed);
    let receipt = host
        .binding("confirm")
        .and_then(|b| b.state().last_receipt)
        .expect("receipt");
    assert_eq!(receipt.status, ReceiptStatus::Applied);
}

// ---------------------------------------------------------------------
// Validation — rejected inputs emit nothing
// ---------------------------------------------------------------------

#[test]
fn dispatch_action_rejects_invalid_inputs() {
    let mut host = host_with(vec![button("confirm")]);
    let generation = host.generation();

    // Unmounted component key.
    let err = host
        .dispatch_action(
            generation,
            &key("unmounted"),
            "view".to_string(),
            "confirm",
            Some(sample_params()),
        )
        .unwrap_err();
    assert!(matches!(err, ViewError::UnknownComponent { .. }));

    // Stale generation.
    let err = host
        .dispatch_action(
            generation + 1,
            &key("confirm"),
            "view".to_string(),
            "confirm",
            None,
        )
        .unwrap_err();
    assert!(matches!(err, ViewError::StaleGeneration { .. }));

    // Non-finite params — `CommandParams::to_value` declines, so the
    // host reports InvalidInput and emits nothing.
    let mut bad = CommandParams::new();
    bad.insert("depth", CommandParam::Number(f64::NAN));
    let err = host
        .dispatch_action(
            generation,
            &key("confirm"),
            "view".to_string(),
            "confirm",
            Some(bad),
        )
        .unwrap_err();
    assert!(matches!(err, ViewError::InvalidInput { .. }));
    assert!(
        !host
            .drain_events()
            .iter()
            .any(|e| matches!(e, ViewEvent::Intent { .. })),
        "rejected input must emit no intent"
    );
}

// ---------------------------------------------------------------------
// Non-regression — params-less intents keep the v1 wire shape
// ---------------------------------------------------------------------

/// `params: None` keeps `"params"` off the wire entirely
/// (`skip_serializing_if`), and `Some(empty)` is a distinct explicit
/// `{}` — an old consumer sees exactly what it saw before.
#[test]
fn paramsless_action_wire_shape_unchanged() {
    let mut host = host_with(vec![button("confirm"), label("status")]);
    let generation = host.generation();

    host.dispatch_action(
        generation,
        &key("confirm"),
        "view".to_string(),
        "confirm",
        None,
    )
    .expect("dispatch_action");
    let (intent, _, expect_receipt) = minted_intent(&mut host);
    assert!(expect_receipt);
    let wire = serde_json::to_value(&intent).expect("serialize intent");
    assert_eq!(
        wire,
        json!({
            "kind": "action",
            "target": "view",
            "control_id": "confirm",
            "sequence": 1,
            "origin_view": "view-1",
            "action": "confirm",
        })
    );

    // Any mounted component — not only an action-declaring kind — may
    // mint a params intent; admission stays consumer-side.
    host.dispatch_action(
        generation,
        &key("status"),
        "view".to_string(),
        "dismiss",
        Some(CommandParams::new()),
    )
    .expect("dispatch_action on label");
    let (intent, _, _) = minted_intent(&mut host);
    let wire = serde_json::to_value(&intent).expect("serialize intent");
    assert_eq!(wire["params"], json!({}));
    assert_eq!(wire["control_id"], json!("status"));
}
