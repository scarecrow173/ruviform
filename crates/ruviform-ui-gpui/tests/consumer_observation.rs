//! Consumer-side observation of the GPUI driver (Issue #429).
//!
//! Written as an external test crate on purpose: it binds only the public
//! `GpuiObserver` seam and `ruviform-ui` contract types — no crate-private
//! hook and no `gpui` type — so it proves what an AudioLens-style consumer
//! test can observe. It mounts the shared Fake/GPUI parity fixture
//! (`driver-lab/parity/new-kinds.json`) and checks its `expect` section.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ruviform_ui::behavior::{CommandPhase, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpoch, ComponentKey, ViewHost, ViewSpec};
use ruviform_ui::{Intent, WindowId, WindowSpec};
use ruviform_ui_gpui::{DrawnPlan, GpuiObserver, ViewMount};
use serde_json::{Value, json};

const COMMAND_OP: &str = "rvf.ui.command.v1";

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json");
    let text = std::fs::read_to_string(path).expect("read parity fixture");
    serde_json::from_str(&text).expect("parse parity fixture")
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

fn assert_near(actual: f64, expected: Option<f64>) {
    let e = expected.expect("fixture number");
    assert!((actual - e).abs() < 1e-3, "{actual} != {e}");
}

/// Mounts `spec` in a new window and settles one turn so the window
/// opens and the view renders.
fn mount_view(observer: &mut GpuiObserver, spec: ViewSpec) -> WindowId {
    let mut host = ViewHost::new(
        ViewInstanceId::new(429).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(spec).expect("mount view");
    host.drain_events();
    let window = observer
        .driver_mut()
        .create_view_window(&WindowSpec::default(), ViewMount { host, theme: None })
        .expect("create_view_window");
    observer.settle(|_, _| {});
    window
}

fn mount_parity(observer: &mut GpuiObserver, fx: &Value) -> WindowId {
    let spec: ViewSpec = serde_json::from_value(fx["view"].clone()).expect("ViewSpec");
    mount_view(observer, spec)
}

/// `kind` of every command intent delivered so far, in order.
fn command_kinds(observer: &GpuiObserver) -> Vec<String> {
    observer
        .intents()
        .iter()
        .filter(|i: &&Intent| i.op == COMMAND_OP)
        .filter_map(|i| {
            i.params
                .get("intent")?
                .get("kind")?
                .as_str()
                .map(str::to_string)
        })
        .collect()
}

fn begin(gesture: &str) -> ControlInput {
    ControlInput::Begin {
        gesture_id: gesture.to_string(),
        source: InputSource::from(InputSource::POINTER),
        origin_view: None,
        target: None,
        at: None,
        modifiers: None,
        seq: None,
    }
}

#[test]
fn observes_mounted_projection_of_parity_view() {
    let fx = fixture();
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    let view = observer.observe(window).expect("view observed");
    assert_eq!(view.window, window);
    let mounted: BTreeSet<String> = view.components.iter().map(|c| c.key.0.clone()).collect();
    let expected: BTreeSet<String> = strings(&fx["expect"]["mountedKeys"]).into_iter().collect();
    assert_eq!(mounted, expected);

    let toggle = view
        .components
        .iter()
        .find(|c| c.key == key("toggle"))
        .expect("toggle observed");
    assert!(toggle.snapshot.mounted);
    assert_eq!(toggle.snapshot.phase, CommandPhase::Idle);
    assert_eq!(
        toggle.snapshot.committed.map(|v| v.normalized),
        fx["view"]["root"]["children"][0]["descriptor"]["default_value"].as_f64(),
        "committed starts at the descriptor default"
    );
    assert_eq!(view.inbox_len, 0);
}

#[test]
fn observes_knob_gesture_intents_through_seam() {
    let fx = fixture();
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    observer
        .dispatch_input(window, &key("knob"), begin("knob-1"))
        .expect("begin");
    observer
        .dispatch_input(
            window,
            &key("knob"),
            ControlInput::Preview {
                gesture_id: "knob-1".to_string(),
                value: ValueInput::Normalized(0.8),
                modifiers: None,
                seq: None,
            },
        )
        .expect("preview");
    observer
        .dispatch_input(
            window,
            &key("knob"),
            ControlInput::Commit {
                gesture_id: "knob-1".to_string(),
                value: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("commit");
    observer.settle(|_, _| {});

    assert_eq!(
        command_kinds(&observer),
        strings(&fx["expect"]["knob"]["intents"])
    );
    let knob = observer.observe(window).expect("observed");
    let committed = knob
        .components
        .iter()
        .find(|c| c.key == key("knob"))
        .and_then(|c| c.snapshot.committed)
        .map(|v| v.normalized);
    assert_eq!(committed, Some(0.8));
}

#[test]
fn observes_toggle_atomic_set_through_seam() {
    let fx = fixture();
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    observer
        .dispatch_input(
            window,
            &key("toggle"),
            ControlInput::Set {
                value: ValueInput::Normalized(1.0),
                gesture_id: None,
                source: Some(InputSource::from(InputSource::POINTER)),
                modifiers: None,
                seq: None,
            },
        )
        .expect("set");
    observer.settle(|_, _| {});

    assert_eq!(
        command_kinds(&observer),
        strings(&fx["expect"]["toggle"]["intents"])
    );
    let toggle = observer.observe(window).expect("observed");
    let committed = toggle
        .components
        .iter()
        .find(|c| c.key == key("toggle"))
        .and_then(|c| c.snapshot.committed)
        .map(|v| v.normalized);
    assert_eq!(
        committed,
        fx["expect"]["toggle"]["committedAfterClick"].as_f64()
    );
}

/// Drawn Meter bars and `TimeAxis` marks equal the shared fixture's
/// `expect` plans (Issue #436). Only the drawing components appear.
#[test]
fn observes_drawn_meter_and_axis_plans_through_seam() {
    let fx = fixture();
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    let drawn = observer.drawn(window).expect("view drawn");
    assert_eq!(drawn.window, window);
    let keys: Vec<ComponentKey> = drawn.components.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, vec![key("meter"), key("ruler")]);
    let plan = |name: &str| {
        drawn
            .components
            .iter()
            .find(|c| c.key == key(name))
            .expect("drawn component")
            .plan
            .clone()
    };

    let DrawnPlan::Meter { bars } = plan("meter") else {
        panic!("meter plan");
    };
    let want_bars = fx["expect"]["meter"]["drawnBars"].as_array().expect("bars");
    assert_eq!(bars.len(), want_bars.len(), "drawn bar count");
    for (bar, want) in bars.iter().zip(want_bars) {
        assert_eq!(u64::try_from(bar.channel).ok(), want["channel"].as_u64());
        assert_near(bar.x0, want["x0"].as_f64());
        assert_near(bar.x1, want["x1"].as_f64());
        assert_near(bar.peak, want["peak"].as_f64());
        assert_near(bar.rms.expect("rms"), want["rms"].as_f64());
    }

    let DrawnPlan::TimeAxis { marks } = plan("ruler") else {
        panic!("time axis plan");
    };
    let want_marks = fx["expect"]["timeAxis"]["drawnMarks"]
        .as_array()
        .expect("marks");
    assert_eq!(marks.len(), want_marks.len(), "drawn mark count");
    for (mark, want) in marks.iter().zip(want_marks) {
        assert_near(mark.x, want["x"].as_f64());
        assert_eq!(Some(mark.major), want["major"].as_bool(), "major");
        assert_eq!(Some(mark.label.as_str()), want["label"].as_str(), "label");
    }
}

/// The parity fixture has no Waveform, so this mounts an inline one. Gap
/// bins (`count == 0`) are absent from the painted bins of each channel.
#[test]
fn observes_drawn_waveform_bins_through_seam() {
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [{
                "type": "waveform", "key": "wave",
                "sample_rate": 48000, "channels": 2, "frames": 300,
                "viewport": { "start": 0, "end": 300 },
                "channel_bins": [
                    [
                        { "start": 0, "end": 100, "min": -0.5, "max": 0.5, "rms": 0.2, "count": 100 },
                        { "start": 100, "end": 200, "min": 0.0, "max": 0.0, "rms": 0.0, "count": 0 }
                    ],
                    [
                        { "start": 0, "end": 300, "min": -1.0, "max": 1.0, "rms": 0.5, "count": 300 }
                    ]
                ],
                "visual": { "error": null }, "a11y": {}
            }]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let drawn = observer.drawn(window).expect("view drawn");
    let [component] = drawn.components.as_slice() else {
        panic!("one waveform component");
    };
    assert_eq!(component.key, key("wave"));
    let DrawnPlan::Waveform { channels } = &component.plan else {
        panic!("waveform plan");
    };
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].channel, 0);
    assert_eq!(channels[0].bins.len(), 1, "gap bin is absent");
    assert_eq!(channels[0].bins[0], 0..100);
    assert_eq!(channels[1].channel, 1);
    assert_eq!(channels[1].bins.len(), 1);
    assert_eq!(channels[1].bins[0], 0..300);
}

#[test]
fn unknown_window_has_no_view_to_observe_or_drive() {
    let observer = GpuiObserver::new();
    let window = WindowId::new(9_999).expect("nonzero");
    assert!(observer.observe(window).is_none());
    assert!(observer.drawn(window).is_none());
    assert!(
        observer
            .dispatch_input(window, &key("knob"), begin("g"))
            .is_err()
    );
}
