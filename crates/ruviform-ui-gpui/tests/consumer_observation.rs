//! Consumer-side observation of the GPUI driver (Issue #429).
//!
//! Written as an external test crate on purpose: it binds only the public
//! `GpuiObserver` seam and `ruviform-ui` contract types — no crate-private
//! hook and no `gpui` type — so it proves what an AudioLens-style consumer
//! test can observe. It mounts the shared Fake/GPUI parity fixture
//! (`driver-lab/parity/new-kinds.json`) and checks its `expect` section.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ruviform_ui::behavior::{CommandPhase, ControlInput, InputSource, Revision, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpoch, ComponentKey, ViewHost, ViewSpec};
use ruviform_ui::{
    DataRegistry, DemoService, Event, Intent, UiRuntime, WindowDriver, WindowId, WindowSpec,
};
use ruviform_ui_gpui::{DrawnPlan, GpuiObserver, ViewDrawn, ViewMount};
use serde_json::{Value, json};

const COMMAND_OP: &str = "rvf.ui.command.v1";

/// Loads a shared Fake/GPUI parity fixture by its file name.
fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ruviform-ui-test/fixtures/driver-lab/parity")
        .join(name);
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

/// The drawn plan of the component `name` in `drawn`.
fn drawn_plan(drawn: &ViewDrawn, name: &str) -> DrawnPlan {
    drawn
        .components
        .iter()
        .find(|c| c.key == key(name))
        .expect("drawn component")
        .plan
        .clone()
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
    let fx = fixture("new-kinds.json");
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
    let fx = fixture("new-kinds.json");
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
    let fx = fixture("new-kinds.json");
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
    let fx = fixture("new-kinds.json");
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    let drawn = observer.drawn(window).expect("view drawn");
    assert_eq!(drawn.window, window);
    let keys: Vec<ComponentKey> = drawn.components.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, vec![key("meter"), key("ruler"), key("spectrogram")]);

    let DrawnPlan::Meter { bars } = drawn_plan(&drawn, "meter") else {
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

    let DrawnPlan::TimeAxis { marks } = drawn_plan(&drawn, "ruler") else {
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

/// Drawn Spectrogram cells equal the shared fixture's `drawnCells`
/// (Issue #442). The gap column and the culled cells draw nothing.
#[test]
fn observes_drawn_spectrogram_cells_through_seam() {
    let fx = fixture("new-kinds.json");
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    let drawn = observer.drawn(window).expect("view drawn");
    let DrawnPlan::Spectrogram { cells } = drawn_plan(&drawn, "spectrogram") else {
        panic!("spectrogram plan");
    };
    let want_cells = fx["expect"]["spectrogram"]["drawnCells"]
        .as_array()
        .expect("cells");
    assert_eq!(cells.len(), want_cells.len(), "drawn cell count");
    for (cell, want) in cells.iter().zip(want_cells) {
        assert_near(cell.x0, want["x0"].as_f64());
        assert_near(cell.x1, want["x1"].as_f64());
        assert_near(cell.y0, want["y0"].as_f64());
        assert_near(cell.y1, want["y1"].as_f64());
        assert_near(cell.intensity, want["intensity"].as_f64());
    }
}

/// Drawn Label text and Table grid equal the Label/Table parity fixture
/// (Issue #442). A missing cell stays `None` in its declared position.
#[test]
fn observes_drawn_label_and_table_plans_through_seam() {
    let fx = fixture("label-table.json");
    let spec: ViewSpec = serde_json::from_value(fx["view"].clone()).expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let drawn = observer.drawn(window).expect("view drawn");
    let keys: Vec<ComponentKey> = drawn.components.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, vec![key("status"), key("events")]);

    let DrawnPlan::Label { text } = drawn_plan(&drawn, "status") else {
        panic!("label plan");
    };
    assert_eq!(Some(text.as_str()), fx["expect"]["label"]["text"].as_str());

    let DrawnPlan::Table { header, rows } = drawn_plan(&drawn, "events") else {
        panic!("table plan");
    };
    let want = &fx["expect"]["table"];
    assert_eq!(header, strings(&want["drawnHeader"]));
    let want_rows: Vec<Vec<Option<String>>> = want["drawnRows"]
        .as_array()
        .expect("drawnRows")
        .iter()
        .map(|row| {
            row.as_array()
                .expect("row")
                .iter()
                .map(|cell| cell.as_str().map(str::to_string))
                .collect()
        })
        .collect();
    assert_eq!(rows, want_rows);
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

/// `Action` intents delivered so far, as the fixture's
/// `{kind, target, controlId, action, params}` summary.
fn action_summaries(observer: &GpuiObserver) -> Value {
    Value::Array(
        observer
            .intents()
            .iter()
            .filter(|i| i.op == COMMAND_OP)
            .filter_map(|i| i.params.get("intent").cloned())
            .filter(|v| v["kind"] == "action")
            .map(|v| {
                json!({
                    "kind": v["kind"],
                    "target": v["target"],
                    "controlId": v["control_id"],
                    "action": v["action"],
                    "params": v["params"],
                })
            })
            .collect(),
    )
}

/// Issue #448: a row press and a Button press emit the Fake side's
/// `Action` intents, and the observed snapshot and drawn plans equal the
/// shared selection/command fixture's `expect`.
#[test]
fn observes_selection_and_command_intents_through_seam() {
    let fx = fixture("selection-command.json");
    let spec: ViewSpec = serde_json::from_value(fx["view"].clone()).expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let row = usize::try_from(fx["expect"]["table"]["selectRow"].as_u64().expect("row"))
        .expect("row index");
    observer
        .select_row(window, &key("jobs"), row)
        .expect("select row");
    observer.invoke(window, &key("run")).expect("invoke");
    let tab = usize::try_from(fx["expect"]["tabs"]["selectIndex"].as_u64().expect("tab"))
        .expect("tab index");
    observer
        .select_tab(window, &key("mixer_tabs"), tab)
        .expect("select tab");
    observer.settle(|_, _| {});

    let mut want = fx["expect"]["table"]["intents"]
        .as_array()
        .expect("table intents")
        .clone();
    want.extend(
        fx["expect"]["button"]["intents"]
            .as_array()
            .expect("button intents")
            .iter()
            .cloned(),
    );
    want.extend(
        fx["expect"]["tabs"]["intents"]
            .as_array()
            .expect("tabs intents")
            .iter()
            .cloned(),
    );
    assert_eq!(action_summaries(&observer), Value::Array(want));

    let view = observer.observe(window).expect("view observed");
    let snap_of = |name: &str| {
        view.components
            .iter()
            .find(|c| c.key == key(name))
            .expect("observed component")
            .snapshot
            .clone()
    };
    let jobs = snap_of("jobs");
    assert_eq!(
        jobs.selected.and_then(|r| u64::try_from(r).ok()),
        fx["expect"]["table"]["selectedAfterSelect"].as_u64()
    );
    assert_eq!(
        serde_json::to_value(jobs.phase).expect("phase json"),
        fx["expect"]["table"]["phase"]
    );
    assert_eq!(
        serde_json::to_value(snap_of("run").phase).expect("phase json"),
        fx["expect"]["button"]["phase"]
    );

    let drawn = observer.drawn(window).expect("view drawn");
    let DrawnPlan::Button { label } = drawn_plan(&drawn, "run") else {
        panic!("button plan");
    };
    assert_eq!(
        Some(label.as_str()),
        fx["expect"]["button"]["drawnLabel"].as_str()
    );
    let DrawnPlan::Table { header, .. } = drawn_plan(&drawn, "jobs") else {
        panic!("table plan");
    };
    assert_eq!(header, strings(&fx["expect"]["table"]["drawnHeader"]));

    let tabs = snap_of("mixer_tabs");
    assert_eq!(
        tabs.selected.and_then(|r| u64::try_from(r).ok()),
        fx["expect"]["tabs"]["selectedAfterSelect"].as_u64()
    );
    assert_eq!(
        serde_json::to_value(tabs.phase).expect("phase json"),
        fx["expect"]["tabs"]["phase"]
    );
    let DrawnPlan::Label { text } = drawn_plan(&drawn, "routing.body") else {
        panic!("selected tab body plan");
    };
    assert_eq!(
        Some(text.as_str()),
        fx["expect"]["tabs"]["drawnBody"].as_str()
    );
    let undrawn = fx["expect"]["tabs"]["undrawnBody"]
        .as_str()
        .expect("undrawn");
    assert!(
        !drawn
            .components
            .iter()
            .any(|c| matches!(&c.plan, DrawnPlan::Label { text } if text.as_str() == undrawn)),
        "the unselected tab body is not drawn"
    );
}

/// Issue #451 FU-1: a key press on the focused tab strip emits the
/// `tab-select` intent the fixture's keyboard step expects, and the observed
/// selection follows it.
#[test]
fn observes_tab_key_intents_through_seam() {
    let fx = fixture("selection-command.json");
    let spec: ViewSpec = serde_json::from_value(fx["view"].clone()).expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);
    let kb = &fx["expect"]["tabs"]["keyboard"];
    let selected = |observer: &GpuiObserver| {
        observer
            .observe(window)
            .expect("view observed")
            .components
            .iter()
            .find(|c| c.key == key("mixer_tabs"))
            .expect("tabs observed")
            .snapshot
            .selected
            .and_then(|r| u64::try_from(r).ok())
    };
    assert_eq!(selected(&observer), kb["fromIndex"].as_u64());

    observer
        .press_tab_key(window, &key("mixer_tabs"), kb["key"].as_str().expect("key"))
        .expect("press tab key");
    observer.settle(|_, _| {});

    assert_eq!(action_summaries(&observer), kb["intents"]);
    assert_eq!(selected(&observer), kb["selectedAfterKey"].as_u64());
}

/// Phase of the fixture's `run` button in the observed projection.
fn run_phase(observer: &GpuiObserver, window: WindowId) -> Value {
    let view = observer.observe(window).expect("view observed");
    let run = view
        .components
        .iter()
        .find(|c| c.key == key("run"))
        .expect("run observed");
    serde_json::to_value(run.snapshot.phase).expect("phase json")
}

/// Posts `events` into the window inbox (the consumer's `post_event`
/// path), settles one turn, and returns the `run` phase afterwards.
fn deliver(observer: &mut GpuiObserver, window: WindowId, events: &[Event]) -> Value {
    for event in events {
        observer.driver_mut().post_event(window, event);
    }
    observer.settle(|_, _| {});
    run_phase(observer, window)
}

/// Issue #456: presses `run` and delivers the runtime's events for that
/// command through the inbox, returning the phase after each step.
fn job_command_phases(op: &str, params: Value) -> Vec<Value> {
    let fx = fixture("selection-command.json");
    let spec: ViewSpec = serde_json::from_value(fx["view"].clone()).expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);
    observer.invoke(window, &key("run")).expect("invoke");
    observer.settle(|_, _| {});
    let command_id = observer
        .intents()
        .iter()
        .rev()
        .find(|i| i.op == COMMAND_OP)
        .expect("command intent")
        .id
        .clone();

    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(DemoService::new())
        .expect("demo service");
    let mut phases = vec![run_phase(&observer, window)];
    phases.push(deliver(
        &mut observer,
        window,
        &rt.dispatch_op(command_id, op, params),
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut terminal = Vec::new();
    while rt.has_running_jobs() && Instant::now() < deadline {
        terminal.extend(rt.poll(Some(Duration::from_millis(50))));
    }
    terminal.extend(rt.drain_jobs());
    phases.push(deliver(&mut observer, window, &terminal));
    phases
}

/// Issue #456: the GPUI side walks the same receipt phases as the Fake
/// side (`ruviform-ui-test/tests/driver_parity.rs`): `pending` → `accepted`
/// on the ack, then `applied` or `rejected` on the job's terminal receipt.
#[test]
fn observes_job_command_receipt_phases_through_seam() {
    let fx = fixture("selection-command.json");
    let want = &fx["expect"]["button"];
    let (pending, ack) = (want["phase"].clone(), want["jobReceipt"]["ack"].clone());
    assert_eq!(
        job_command_phases("demo.sleep", json!({ "ms": 10 })),
        [
            pending.clone(),
            ack.clone(),
            want["jobReceipt"]["applied"].clone()
        ]
    );
    assert_eq!(
        job_command_phases("demo.panic", json!({})),
        [pending, ack, want["jobReceipt"]["rejected"].clone()]
    );
}

// ── Issue #429 remaining DoD ────────────────────────────────────────
//
// Bound-near payloads, projections landing mid-gesture, spec snapshot
// replacement, and f64 precision — all observed through the public seam
// only. Gap/NaN handling, bound rejection, cancel/stale/out-of-order,
// and teardown are covered by the lanes enumerated in the task doc and
// are not re-implemented here.

/// Fraction of `viewport` at which `at` falls — mirrors the renderer's
/// `viewport_fraction` (f64 → f32 → f64) so expected values compare
/// bit-exact against the observed `f64`s.
fn viewport_frac(at: u64, start: u64, end: u64) -> f64 {
    let span = (end - start).max(1) as f64;
    f64::from(((at as f64 - start as f64) / span) as f32)
}

/// Spectrogram cell intensity for `magnitude` inside `lo..hi` — mirrors
/// `spectrogram_cell_plan`: f64 math, clamp, then f32 → f64 widening.
fn cell_intensity(magnitude: f64, lo: f64, hi: f64) -> f64 {
    f64::from(((magnitude - lo) / (hi - lo)).clamp(0.0, 1.0) as f32)
}

/// Top-down y span of `row` in a `rows`-high spectrogram — mirrors the
/// plan's f32 arithmetic widened to f64.
fn cell_y(row: usize, rows: usize) -> (f64, f64) {
    let rows = rows.max(1) as f32;
    (
        f64::from(1.0 - (row + 1) as f32 / rows),
        f64::from(1.0 - row as f32 / rows),
    )
}

/// Peak of meter bin `i` on channel `c`: mostly in `[0, 1]`, with a
/// deterministic over-range value (`> 1.0`) every 53 bins so the seam
/// reports the renderer's clamp.
fn meter_peak(channel: usize, bin: usize) -> f64 {
    if (channel + bin) % 53 == 0 {
        1.5
    } else {
        ((channel * 31 + bin * 7) % 17) as f64 / 16.0
    }
}

/// `MAX_WAVEFORM_BINS` is 16_384: 8 channels × 2_048 bins sits exactly on
/// the bound. Frame numbers beyond `u32::MAX` prove the u64 → f64 path
/// keeps long-duration ranges bit-exact through the seam.
#[test]
fn observes_waveform_at_bin_bound_with_long_frame_range() {
    const CHANNELS: usize = 8;
    const BINS_PER_CHANNEL: usize = 2048;
    const BIN_SPAN: u64 = 8;
    let viewport_start = 4_999_984_000u64;
    let viewport_end = viewport_start + BINS_PER_CHANNEL as u64 * BIN_SPAN;

    let channel_bins = Value::Array(
        (0..CHANNELS)
            .map(|_| {
                Value::Array(
                    (0..BINS_PER_CHANNEL)
                        .map(|i| {
                            let start = viewport_start + i as u64 * BIN_SPAN;
                            json!({
                                "start": start,
                                "end": start + BIN_SPAN,
                                "min": -1.0,
                                "max": 1.0,
                                "rms": 0.5,
                                "count": 100,
                            })
                        })
                        .collect(),
                )
            })
            .collect(),
    );
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [{
                "type": "waveform", "key": "wave",
                "sample_rate": 48000, "channels": CHANNELS,
                "frames": viewport_end,
                "viewport": { "start": viewport_start, "end": viewport_end },
                "channel_bins": channel_bins,
                "visual": { "error": null }, "a11y": {}
            }]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let view = observer.observe(window).expect("view observed");
    assert!(
        view.components
            .iter()
            .any(|c| c.key == key("wave") && c.snapshot.mounted)
    );

    let drawn = observer.drawn(window).expect("view drawn");
    let [component] = drawn.components.as_slice() else {
        panic!("one waveform component");
    };
    let DrawnPlan::Waveform { channels } = &component.plan else {
        panic!("waveform plan");
    };
    assert_eq!(channels.len(), CHANNELS);
    for (channel, drawn_channel) in channels.iter().enumerate() {
        assert_eq!(drawn_channel.channel, channel);
        assert_eq!(drawn_channel.bins.len(), BINS_PER_CHANNEL);
        for (i, bin) in drawn_channel.bins.iter().enumerate() {
            let start = viewport_start + i as u64 * BIN_SPAN;
            assert_eq!(*bin, start..start + BIN_SPAN);
        }
    }
}

/// `MAX_SPECTROGRAM_CELLS` is 65_536: 512 columns × 128 rows sits exactly
/// on the bound. The viewport keeps columns 128..=383 (256 columns), so
/// 32_768 cells paint — column-major order, f32-exact geometry.
#[test]
fn observes_spectrogram_at_cell_bound_with_viewport_culling() {
    const ROWS: usize = 128;
    const COLUMNS: usize = 512;
    const VIEWPORT: (u64, u64) = (128, 384);
    const KEPT_COLUMNS: usize = 256;

    // Deterministic magnitudes in `[-60, 0)` — repeating-decimal
    // intensities exercise the f64 → f32 → f64 round-trip.
    let magnitude =
        |column: usize, row: usize| -> f64 { -60.0 + ((column * 31 + row * 17) % 61) as f64 };
    let columns = Value::Array(
        (0..COLUMNS)
            .map(|i| {
                json!({
                    "start": i as u64,
                    "end": i as u64 + 1,
                    "count": 100,
                    "magnitudes": (0..ROWS).map(|r| magnitude(i, r)).collect::<Vec<_>>(),
                })
            })
            .collect(),
    );
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [{
                "type": "spectrogram", "key": "spec",
                "viewport": { "start": VIEWPORT.0, "end": VIEWPORT.1 },
                "rows": ROWS,
                "magnitude_range": { "start": -60.0, "end": 0.0 },
                "columns": columns,
                "visual": { "error": null }, "a11y": {}
            }]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let drawn = observer.drawn(window).expect("view drawn");
    let DrawnPlan::Spectrogram { cells } = drawn_plan(&drawn, "spec") else {
        panic!("spectrogram plan");
    };
    assert_eq!(
        cells.len(),
        KEPT_COLUMNS * ROWS,
        "culled columns draw nothing"
    );
    for (kept, column) in (VIEWPORT.0 as usize..VIEWPORT.1 as usize).enumerate() {
        for (row, cell) in cells[kept * ROWS..(kept + 1) * ROWS].iter().enumerate() {
            let (y0, y1) = cell_y(row, ROWS);
            assert_eq!(
                cell.x0,
                viewport_frac(column as u64, VIEWPORT.0, VIEWPORT.1).max(0.0)
            );
            assert_eq!(
                cell.x1,
                viewport_frac(column as u64 + 1, VIEWPORT.0, VIEWPORT.1).min(1.0)
            );
            assert_eq!(cell.y0, y0);
            assert_eq!(cell.y1, y1);
            assert_eq!(
                cell.intensity,
                cell_intensity(magnitude(column, row), -60.0, 0.0)
            );
        }
    }
}

/// `MAX_METER_BINS` is 8_192: 16 channels × 512 bins sits exactly on the
/// bound. Bins fully outside the viewport are culled per channel — bin
/// `i` covers `[i*16, (i+1)*16)`, so bins 62..=312 (251 per channel)
/// intersect viewport `1000..5000`.
#[test]
fn observes_meter_at_bin_bound_with_viewport_culling() {
    const CHANNELS: usize = 16;
    const BINS_PER_CHANNEL: usize = 512;
    const BIN_SPAN: u64 = 16;
    const VIEWPORT: (u64, u64) = (1000, 5000);
    const KEPT_FIRST: usize = 62;
    const KEPT_BINS: usize = 251;

    let channel_levels = Value::Array(
        (0..CHANNELS)
            .map(|c| {
                Value::Array(
                    (0..BINS_PER_CHANNEL)
                        .map(|i| {
                            let start = i as u64 * BIN_SPAN;
                            let peak = meter_peak(c, i);
                            json!({
                                "start": start,
                                "end": start + BIN_SPAN,
                                "peak": peak,
                                "rms": peak * 0.5,
                                "count": 1000,
                            })
                        })
                        .collect(),
                )
            })
            .collect(),
    );
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [{
                "type": "meter", "key": "meter",
                "channels": CHANNELS,
                "channel_levels": channel_levels,
                "viewport": { "start": VIEWPORT.0, "end": VIEWPORT.1 },
                "visual": { "error": null }, "a11y": {}
            }]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    let drawn = observer.drawn(window).expect("view drawn");
    let DrawnPlan::Meter { bars } = drawn_plan(&drawn, "meter") else {
        panic!("meter plan");
    };
    assert_eq!(bars.len(), CHANNELS * KEPT_BINS);
    for (channel, channel_bars) in bars.chunks(KEPT_BINS).enumerate() {
        assert_eq!(channel_bars.len(), KEPT_BINS);
        for (j, bar) in channel_bars.iter().enumerate() {
            let i = KEPT_FIRST + j;
            let start = i as u64 * BIN_SPAN;
            let peak = meter_peak(channel, i);
            assert_eq!(bar.channel, channel);
            assert_eq!(
                bar.x0,
                viewport_frac(start, VIEWPORT.0, VIEWPORT.1).max(0.0)
            );
            assert_eq!(
                bar.x1,
                viewport_frac(start + BIN_SPAN, VIEWPORT.0, VIEWPORT.1).min(1.0)
            );
            assert_eq!(bar.peak, f64::from(peak.clamp(0.0, 1.0) as f32));
            assert_eq!(
                bar.rms,
                Some(f64::from((peak * 0.5).clamp(0.0, 1.0) as f32))
            );
        }
    }
}

/// A `Notify` projection that lands mid-gesture is recorded as
/// `observed` and staged, while the local preview keeps driving
/// `display`; the staged truth becomes `committed` when the gesture
/// commits — all visible through the seam.
#[test]
fn observes_projection_during_gesture_through_seam() {
    let fx = fixture("new-kinds.json");
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    observer
        .dispatch_input(window, &key("knob"), begin("g1"))
        .expect("begin");
    observer
        .dispatch_input(
            window,
            &key("knob"),
            ControlInput::Preview {
                gesture_id: "g1".to_string(),
                value: ValueInput::Normalized(0.8),
                modifiers: None,
                seq: None,
            },
        )
        .expect("preview");

    // Two authoritative pushes land while the gesture is active; the
    // newer revision wins `observed`, the preview keeps `display`.
    for (seq, revision, normalized) in [(1u64, 1u64, 0.3), (2, 2, 0.9)] {
        observer.driver_mut().post_event(
            window,
            &Event::Notify {
                seq,
                data: json!({
                    "control_id": "knob",
                    "revision": revision,
                    "value": { "normalized": normalized },
                }),
            },
        );
        observer.settle(|_, _| {});
        let view = observer.observe(window).expect("view observed");
        let knob = view
            .components
            .iter()
            .find(|c| c.key == key("knob"))
            .expect("knob observed");
        let snap = &knob.snapshot;
        assert_eq!(snap.phase, CommandPhase::Preview);
        assert_eq!(snap.display.map(|v| v.normalized), Some(0.8));
        assert_eq!(snap.committed.map(|v| v.normalized), Some(0.5));
        assert_eq!(
            snap.observed.as_ref().map(|p| p.revision.clone()),
            Some(Revision::Num(revision))
        );
    }

    observer
        .dispatch_input(
            window,
            &key("knob"),
            ControlInput::Commit {
                gesture_id: "g1".to_string(),
                value: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("commit");
    observer.settle(|_, _| {});
    let view = observer.observe(window).expect("view observed");
    let snap = &view
        .components
        .iter()
        .find(|c| c.key == key("knob"))
        .expect("knob observed")
        .snapshot;
    // The last staged projection is the authoritative truth — it
    // overwrites even the just-committed preview value.
    assert_eq!(snap.display.map(|v| v.normalized), Some(0.9));
    assert_eq!(snap.committed.map(|v| v.normalized), Some(0.9));
}

/// Contract f64 values survive the whole observe path: `committed` and
/// `display` are never narrowed through f32. `Plain` input round-trips
/// bit-exact; `Normalized` input resolves through the same f64 mapper
/// ops the renderer-independent contract defines.
#[test]
fn observes_committed_value_at_f64_precision() {
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [{
                "type": "knob", "key": "fine",
                "control_id": "fine", "target": null,
                "descriptor": {
                    "min": -60.0, "max": 0.0, "default_value": -30.0,
                    "step": null, "step_domain": null,
                    "fine_step": null, "fine_step_domain": null,
                    "mapping": null, "out_of_range": null,
                    "unit": null, "read_only": false,
                    "disabled": false, "constant": false
                },
                "orientation": "vertical", "detach_policy": "cancel",
                "unsupported": false,
                "visual": { "error": null },
                "a11y": { "role": "slider", "label": "fine" }
            }]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);
    let snap = |observer: &GpuiObserver| {
        observer
            .observe(window)
            .expect("view observed")
            .components
            .iter()
            .find(|c| c.key == key("fine"))
            .expect("fine observed")
            .snapshot
            .clone()
    };

    let plain = -33.333333333333336;
    observer
        .dispatch_input(
            window,
            &key("fine"),
            ControlInput::Set {
                value: ValueInput::Plain(plain),
                gesture_id: None,
                source: Some(InputSource::from(InputSource::POINTER)),
                modifiers: None,
                seq: None,
            },
        )
        .expect("set plain");
    observer.settle(|_, _| {});
    let s = snap(&observer);
    assert_eq!(
        s.committed.map(|v| v.plain.to_bits()),
        Some(plain.to_bits()),
        "plain round-trips bit-exact"
    );
    assert_eq!(
        s.committed.map(|v| v.normalized),
        Some((plain + 60.0) / 60.0)
    );
    assert_eq!(s.display.map(|v| v.plain.to_bits()), Some(plain.to_bits()));

    let n = 0.7543210987654321;
    observer
        .dispatch_input(
            window,
            &key("fine"),
            ControlInput::Set {
                value: ValueInput::Normalized(n),
                gesture_id: None,
                source: Some(InputSource::from(InputSource::POINTER)),
                modifiers: None,
                seq: None,
            },
        )
        .expect("set normalized");
    observer.settle(|_, _| {});
    let s = snap(&observer);
    // `min + n * (max - min)` in f64 — the same op order as the mapper.
    let want_plain = -60.0 + n * 60.0;
    assert_eq!(
        s.committed.map(|v| v.plain.to_bits()),
        Some(want_plain.to_bits())
    );
    assert_eq!(s.committed.map(|v| v.normalized), Some(n));
    assert_eq!(s.display.map(|v| v.normalized), Some(n));
}

/// `update_view` replaces the mounted spec; the seam then observes the
/// advanced generation and the re-projected drawing plan — the
/// snapshot-push path a consumer drives at RT cadence.
#[test]
fn observes_updated_spec_reprojection_through_seam() {
    let waveform = |end: u64| -> ViewSpec {
        serde_json::from_value(json!({
            "root": {
                "type": "panel", "key": "root", "title": null,
                "visual": { "error": null }, "a11y": {},
                "children": [{
                    "type": "waveform", "key": "wave",
                    "sample_rate": 48000, "channels": 1, "frames": end,
                    "viewport": { "start": 0, "end": end },
                    "channel_bins": [[{
                        "start": 0, "end": end,
                        "min": -1.0, "max": 1.0, "rms": 0.5, "count": 100
                    }]],
                    "visual": { "error": null }, "a11y": {}
                }]
            }
        }))
        .expect("ViewSpec")
    };
    let bins_of = |observer: &GpuiObserver, window: WindowId| {
        let drawn = observer.drawn(window).expect("view drawn");
        let DrawnPlan::Waveform { channels } = drawn_plan(&drawn, "wave") else {
            panic!("waveform plan");
        };
        channels[0].bins.clone()
    };

    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, waveform(100));
    let before = observer.observe(window).expect("view observed");
    assert_eq!(bins_of(&observer, window), vec![0..100]);

    observer
        .driver()
        .update_view(window, waveform(200))
        .expect("update_view");
    observer.settle(|_, _| {});

    let after = observer.observe(window).expect("view observed");
    // Reconcile preserves the mount: the generation guard is stable
    // across spec updates (it advances on mount/detach only).
    assert_eq!(after.generation, before.generation);
    assert_eq!(bins_of(&observer, window), vec![0..200]);
}

/// Issue #451 FU-1b: Tab / Shift-Tab traverse the window's tab stops
/// through the driver's app-level keystroke observer — forward,
/// backward with wrap-around, no movement from other modifiers, and
/// reaching the first stop from no focus at all.
#[test]
fn tab_and_shift_tab_traverse_window_tab_stops() {
    let tabs = |key: &str, target: &str| {
        json!({
            "type": "tabs", "key": key, "active": 0,
            "tabs": [
                { "key": format!("{key}.one"), "title": "One",
                  "child": { "type": "label", "key": format!("{key}.one.body"), "text": "one",
                             "visual": { "error": null }, "a11y": {} } },
                { "key": format!("{key}.two"), "title": "Two",
                  "child": { "type": "label", "key": format!("{key}.two.body"), "text": "two",
                             "visual": { "error": null }, "a11y": {} } }
            ],
            "selection": { "target": target },
            "visual": { "error": null }, "a11y": {}
        })
    };
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [tabs("tabs_a", "a.select"), tabs("tabs_b", "b.select")]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    // No focus: Tab reaches the first tab stop.
    assert_eq!(observer.focused_tab(window), None);
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_a")));
    // Tab moves to the next strip, then wraps to the first.
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_b")));
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_a")));
    // Shift-Tab walks backwards and wraps to the last stop.
    observer
        .press_key(window, "shift-tab")
        .expect("press shift-tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_b")));
    // Other modifiers do not traverse.
    observer
        .press_key(window, "ctrl-tab")
        .expect("press ctrl-tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_b")));
    observer
        .press_key(window, "alt-tab")
        .expect("press alt-tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_b")));

    // Focus traversal is not selection: no command intent is minted.
    observer.settle(|_, _| {});
    assert!(command_kinds(&observer).is_empty());
}

/// Issue #479: each fixture key step pressed on the focused control
/// emits the same `begin`+`commit` intents and lands on the same
/// committed normalized as the Fake side (`driver_parity.rs`).
#[test]
fn observes_key_op_intents_through_seam() {
    let fx = fixture("key-ops.json");
    let mut observer = GpuiObserver::new();
    let window = mount_parity(&mut observer, &fx);

    let mut want_kinds = Vec::new();
    for (name, spec) in fx["expect"]["controls"]
        .as_object()
        .expect("controls object")
    {
        for step in spec["keys"].as_array().expect("keys") {
            observer
                .press_control_key(window, &key(name), step["key"].as_str().expect("key"))
                .expect("press control key");
            assert_eq!(
                observer.focused_control(window),
                Some(key(name)),
                "{name} holds focus"
            );
        }
        want_kinds.extend(strings(&spec["intents"]));
    }
    observer.settle(|_, _| {});
    assert_eq!(command_kinds(&observer), want_kinds);

    let view = observer.observe(window).expect("view observed");
    for (name, spec) in fx["expect"]["controls"]
        .as_object()
        .expect("controls object")
    {
        let snap = &view
            .components
            .iter()
            .find(|c| c.key == key(name))
            .expect("observed control")
            .snapshot;
        let last = spec["keys"]
            .as_array()
            .expect("keys")
            .last()
            .expect("last step");
        let want = last["committedNormalized"].as_f64().expect("committed");
        let got = snap
            .committed
            .map(|c| c.normalized)
            .expect("committed value");
        assert_near(got, Some(want));
    }
}

/// Issue #479: value controls are tab stops — Tab traversal reaches a
/// Fader after the tab strip and Shift-Tab walks back to it, and the
/// focused control is reported through `focused_control`.
#[test]
fn tab_traversal_reaches_value_controls() {
    let tabs = json!({
        "type": "tabs", "key": "tabs_a", "active": 0,
        "tabs": [
            { "key": "tabs_a.one", "title": "One",
              "child": { "type": "label", "key": "tabs_a.one.body", "text": "one",
                         "visual": { "error": null }, "a11y": {} } }
        ],
        "selection": { "target": "a.select" },
        "visual": { "error": null }, "a11y": {}
    });
    let fader = json!({
        "type": "fader", "key": "fader", "control_id": null,
        "target": "mixer.fader",
        "descriptor": {
            "min": 0.0, "max": 1.0, "default_value": 0.5,
            "step": 0.05, "step_domain": null,
            "fine_step": 0.01, "fine_step_domain": null,
            "mapping": null, "out_of_range": null, "unit": null,
            "read_only": false, "disabled": false, "constant": false
        },
        "orientation": "vertical", "detach_policy": "cancel",
        "unsupported": false, "visual": { "error": null },
        "a11y": { "role": "slider", "label": "fader" }
    });
    let spec: ViewSpec = serde_json::from_value(json!({
        "root": {
            "type": "panel", "key": "root", "title": null,
            "visual": { "error": null }, "a11y": {},
            "children": [tabs, fader]
        }
    }))
    .expect("ViewSpec");
    let mut observer = GpuiObserver::new();
    let window = mount_view(&mut observer, spec);

    // Tab order follows paint order: the tab strip header, then the
    // Fader — and traversal wraps around both stops.
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_a")));
    assert_eq!(observer.focused_control(window), None);
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), None);
    assert_eq!(observer.focused_control(window), Some(key("fader")));
    observer.press_key(window, "tab").expect("press tab");
    assert_eq!(observer.focused_tab(window), Some(key("tabs_a")));
    observer
        .press_key(window, "shift-tab")
        .expect("press shift-tab");
    assert_eq!(observer.focused_control(window), Some(key("fader")));

    // Traversal itself mints no command intent.
    observer.settle(|_, _| {});
    assert!(command_kinds(&observer).is_empty());
}
