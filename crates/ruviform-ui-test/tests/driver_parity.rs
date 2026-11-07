//! Issue #421 Fake/GPUI parity — the fake (headless) side.
//!
//! Reads `fixtures/driver-lab/parity/new-kinds.json`, the same fixture the
//! GPUI side reads in `ruviform-ui-gpui`, and checks the headless
//! semantics of the Knob/Toggle/Meter/TimeAxis slice:
//!
//! - `UI-DVR-02` — mount/reconcile keeps stable keys for the new kinds.
//! - `UI-DVR-03` — Knob gesture and Toggle atomic set mint the expected
//!   intents and committed value.
//! - Payload retention — the headless layer keeps the declared Meter/TimeAxis
//!   payload (gap bins and out-of-viewport ticks included); culling is a
//!   renderer concern checked by the GPUI side against `expect.*.drawn*`.

use std::path::PathBuf;

use ruviform_ui::behavior::{CommandIntent, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, ProjectionOp, ViewEvent, ViewHost, ViewSpec,
};
use serde_json::Value;

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/driver-lab/parity/new-kinds.json");
    let text = std::fs::read_to_string(&path).expect("read parity fixture");
    serde_json::from_str(&text).expect("parse parity fixture")
}

fn view_spec(fx: &Value) -> ViewSpec {
    serde_json::from_value(fx["view"].clone())
        .expect("ViewSpec deserializes from the shared fixture")
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|s| s.as_str().expect("string").to_string())
        .collect()
}

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn new_host() -> ViewHost {
    ViewHost::new(
        ViewInstanceId::new(421).expect("nonzero"),
        AttachEpoch::new(1),
    )
}

/// Kinds of the `CommandIntent`s minted since the last drain.
fn intent_kinds(host: &mut ViewHost) -> Vec<String> {
    host.drain_events()
        .into_iter()
        .filter_map(|e| match e {
            ViewEvent::Intent { intent, .. } => Some(intent),
            _ => None,
        })
        .map(|i: CommandIntent| {
            serde_json::to_value(&i).expect("intent json")["kind"]
                .as_str()
                .expect("kind")
                .to_string()
        })
        .collect()
}

#[test]
fn ui_dvr_02_parity_new_kinds_mount_reconcile() {
    let fx = fixture();
    let mut host = new_host();
    let ops = host.mount(view_spec(&fx)).expect("mount");
    for k in strings(&fx["expect"]["mountedKeys"]) {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(&k) }), "{k}");
    }

    // Reconcile: drop the toggle, keep the knob — the surviving key keeps its
    // identity and the dropped key is removed.
    let mut next = view_spec(&fx);
    if let ComponentSpec::Panel(p) = &mut next.root {
        p.children.retain(|c| c.key().0 != "toggle");
    }
    let generation = host.generation();
    let ops = host.update(next, generation).expect("reconcile");
    assert!(ops.contains(&ProjectionOp::Removed { key: key("toggle") }));
    assert!(host.observe(&key("knob")).is_some());
    assert!(host.observe(&key("toggle")).is_none());
}

#[test]
fn ui_dvr_03_parity_new_kinds_gesture() {
    let fx = fixture();
    let mut host = new_host();
    host.mount(view_spec(&fx)).expect("mount");
    host.drain_events();

    let generation = host.generation();
    let knob = key("knob");
    let begin = ControlInput::Begin {
        gesture_id: "knob-1".to_string(),
        source: InputSource::from(InputSource::POINTER),
        origin_view: None,
        target: None,
        at: None,
        modifiers: None,
        seq: None,
    };
    host.dispatch_input(generation, &knob, begin)
        .expect("begin");
    let preview = ControlInput::Preview {
        gesture_id: "knob-1".to_string(),
        value: ValueInput::Normalized(0.8),
        modifiers: None,
        seq: None,
    };
    host.dispatch_input(generation, &knob, preview)
        .expect("preview");
    let commit = ControlInput::Commit {
        gesture_id: "knob-1".to_string(),
        value: None,
        modifiers: None,
        seq: None,
    };
    host.dispatch_input(generation, &knob, commit)
        .expect("commit");
    assert_eq!(
        intent_kinds(&mut host),
        strings(&fx["expect"]["knob"]["intents"])
    );

    // Toggle: one pointer click is one atomic set to the flipped value.
    let set = ControlInput::Set {
        value: ValueInput::Normalized(1.0),
        gesture_id: None,
        source: Some(InputSource::from(InputSource::POINTER)),
        modifiers: None,
        seq: None,
    };
    host.dispatch_input(generation, &key("toggle"), set)
        .expect("set");
    assert_eq!(
        intent_kinds(&mut host),
        strings(&fx["expect"]["toggle"]["intents"])
    );
    let committed = host
        .observe(&key("toggle"))
        .and_then(|s| s.committed)
        .map(|c| c.normalized);
    assert_eq!(
        committed,
        fx["expect"]["toggle"]["committedAfterClick"].as_f64()
    );
}

#[test]
fn parity_new_kinds_payload_retained_fake() {
    let fx = fixture();
    let spec = view_spec(&fx);
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("parity fixture root must be a panel");
    };
    let meter = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Meter(m) => Some(m),
            _ => None,
        })
        .expect("meter");
    let declared_bins: usize = meter.channel_levels.iter().map(Vec::len).sum();
    assert_eq!(
        u64::try_from(declared_bins).expect("small"),
        fx["expect"]["meter"]["declaredBins"]
            .as_u64()
            .expect("declaredBins")
    );
    assert_eq!(
        meter
            .channel_levels
            .iter()
            .flatten()
            .filter(|b| b.count == 0)
            .count(),
        2,
        "gap bins pass through the headless payload"
    );

    let axis = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::TimeAxis(t) => Some(t),
            _ => None,
        })
        .expect("time_axis");
    assert_eq!(
        u64::try_from(axis.ticks.len()).expect("small"),
        fx["expect"]["timeAxis"]["declaredTicks"]
            .as_u64()
            .expect("declaredTicks")
    );
    assert!(
        axis.ticks.iter().any(|t| t.at > 1500.0),
        "out-of-viewport tick is retained"
    );

    let mut host = new_host();
    host.mount(spec)
        .expect("payload accepted by headless validation");
}

#[test]
fn parity_spectrogram_payload_retained_fake() {
    let fx = fixture();
    let spec = view_spec(&fx);
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("parity fixture root must be a panel");
    };
    let spectrogram = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Spectrogram(s) => Some(s),
            _ => None,
        })
        .expect("spectrogram");
    let expect = &fx["expect"]["spectrogram"];
    assert_eq!(
        u64::try_from(spectrogram.columns.len()).expect("small"),
        expect["declaredColumns"].as_u64().expect("declaredColumns")
    );
    let cells: usize = spectrogram.columns.iter().map(|c| c.magnitudes.len()).sum();
    assert_eq!(
        u64::try_from(cells).expect("small"),
        expect["declaredCells"].as_u64().expect("declaredCells")
    );
    assert_eq!(
        u64::try_from(spectrogram.columns.iter().filter(|c| c.count == 0).count()).expect("small"),
        expect["gapColumns"].as_u64().expect("gapColumns"),
        "count==0 columns pass through the headless payload"
    );

    let mut host = new_host();
    let ops = host
        .mount(spec)
        .expect("payload accepted by headless validation");
    assert!(ops.contains(&ProjectionOp::Mounted {
        key: key("spectrogram"),
    }));
    assert!(host.observe(&key("spectrogram")).is_some());
}
