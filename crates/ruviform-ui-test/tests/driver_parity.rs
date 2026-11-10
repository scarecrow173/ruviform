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
use std::time::{Duration, Instant};

use ruviform_ui::behavior::{
    CommandIntent, CommandReceipt, ControlInput, Extreme, InputModifiers, InputSource,
    InteractionEvent, ValueInput,
};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, ProjectionOp, ViewEvent, ViewHost, ViewSpec,
};
use ruviform_ui::{DataRegistry, DemoService, Event, UiRuntime};
use serde_json::{Value, json};

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

fn label_table_fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/driver-lab/parity/label-table.json");
    let text = std::fs::read_to_string(&path).expect("read label-table fixture");
    serde_json::from_str(&text).expect("parse label-table fixture")
}

/// Issue #435: Label/Table mount by key, and a dropped Label leaves the
/// Table's identity intact.
#[test]
fn ui_dvr_02_parity_label_table_mount_reconcile() {
    let fx = label_table_fixture();
    let mut host = new_host();
    let ops = host.mount(view_spec(&fx)).expect("mount");
    for k in strings(&fx["expect"]["mountedKeys"]) {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(&k) }), "{k}");
    }

    let mut next = view_spec(&fx);
    if let ComponentSpec::Panel(p) = &mut next.root {
        p.children.retain(|c| c.key().0 != "status");
    }
    let generation = host.generation();
    let ops = host.update(next, generation).expect("reconcile");
    assert!(ops.contains(&ProjectionOp::Removed { key: key("status") }));
    assert!(host.observe(&key("events")).is_some());
    assert!(host.observe(&key("status")).is_none());
}

/// The headless a11y tree of each Label/Table equals the fixture's
/// `expect.*.a11yTree` (roles, names, header row, cell values, and
/// valueless cells for missing values).
#[test]
fn parity_label_table_a11y_tree_fake() {
    let fx = label_table_fixture();
    let mut host = new_host();
    host.mount(view_spec(&fx)).expect("mount");
    let label = host.a11y_node(&key("status")).expect("label a11y");
    assert_eq!(
        serde_json::to_value(&label).expect("json"),
        fx["expect"]["label"]["a11yTree"]
    );
    let table = host.a11y_node(&key("events")).expect("table a11y");
    assert_eq!(
        serde_json::to_value(&table).expect("json"),
        fx["expect"]["table"]["a11yTree"]
    );
}

/// Column and row order and missing cells (`null`) survive the headless
/// payload, so every row keeps its full cell count.
#[test]
fn parity_label_table_payload_retained_fake() {
    let fx = label_table_fixture();
    let spec = view_spec(&fx);
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("parity fixture root must be a panel");
    };
    let table = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Table(t) => Some(t),
            _ => None,
        })
        .expect("table");
    assert_eq!(
        table.columns,
        strings(&fx["expect"]["table"]["drawnHeader"])
    );
    assert_eq!(
        u64::try_from(table.rows.len()).expect("small"),
        fx["expect"]["table"]["declaredRows"]
            .as_u64()
            .expect("declaredRows")
    );
    assert!(table.rows.iter().all(|r| r.len() == table.columns.len()));
    assert!(table.rows[1][0].is_none(), "missing cell keeps its column");
    assert!(table.rows[2][2].is_none(), "trailing missing cell kept");

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

/// Issue #448: the shared selection/command fixture — the row selection
/// and the Button press emit the same `Action` intents and snapshot as
/// the GPUI side (`consumer_observation.rs`).
fn selection_fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/driver-lab/parity/selection-command.json");
    let text = std::fs::read_to_string(&path).expect("read selection fixture");
    serde_json::from_str(&text).expect("parse selection fixture")
}

/// The `Action` intents minted since the last drain, as the fixture's
/// `{kind, target, controlId, action, params}` summary.
fn action_summaries(host: &mut ViewHost) -> Value {
    Value::Array(
        host.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ViewEvent::Intent {
                    intent: intent @ CommandIntent::Action { .. },
                    ..
                } => Some(intent),
                _ => None,
            })
            .map(|i| {
                let v = serde_json::to_value(&i).expect("intent json");
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

#[test]
fn ui_dvr_02_parity_selection_command_mount() {
    let fx = selection_fixture();
    let mut host = new_host();
    let ops = host.mount(view_spec(&fx)).expect("mount");
    for k in strings(&fx["expect"]["mountedKeys"]) {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(&k) }), "{k}");
    }
    let button = host.a11y_node(&key("run")).expect("button a11y tree");
    assert_eq!(
        serde_json::to_value(&button).expect("a11y json"),
        fx["expect"]["button"]["a11yTree"]
    );
}

#[test]
fn ui_dvr_03_parity_selection_command_intents_and_snapshot() {
    let fx = selection_fixture();
    let mut host = new_host();
    host.mount(view_spec(&fx)).expect("mount");
    host.drain_events();

    let row = usize::try_from(fx["expect"]["table"]["selectRow"].as_u64().expect("row"))
        .expect("row index");
    host.select_row(host.generation(), &key("jobs"), row)
        .expect("select row");
    assert_eq!(
        action_summaries(&mut host),
        fx["expect"]["table"]["intents"]
    );
    let snap = host.observe(&key("jobs")).expect("table snapshot");
    assert_eq!(
        snap.selected.and_then(|r| u64::try_from(r).ok()),
        fx["expect"]["table"]["selectedAfterSelect"].as_u64()
    );
    assert_eq!(
        serde_json::to_value(snap.phase).expect("phase json"),
        fx["expect"]["table"]["phase"]
    );

    host.invoke(host.generation(), &key("run"))
        .expect("invoke button");
    assert_eq!(
        action_summaries(&mut host),
        fx["expect"]["button"]["intents"]
    );
    let snap = host.observe(&key("run")).expect("button snapshot");
    assert_eq!(
        serde_json::to_value(snap.phase).expect("phase json"),
        fx["expect"]["button"]["phase"]
    );

    let index = usize::try_from(fx["expect"]["tabs"]["selectIndex"].as_u64().expect("tab"))
        .expect("tab index");
    host.select_tab(host.generation(), &key("mixer_tabs"), index)
        .expect("select tab");
    assert_eq!(action_summaries(&mut host), fx["expect"]["tabs"]["intents"]);
    let snap = host.observe(&key("mixer_tabs")).expect("tabs snapshot");
    assert_eq!(
        snap.selected.and_then(|r| u64::try_from(r).ok()),
        fx["expect"]["tabs"]["selectedAfterSelect"].as_u64()
    );
    assert_eq!(
        serde_json::to_value(snap.phase).expect("phase json"),
        fx["expect"]["tabs"]["phase"]
    );
}

/// Issue #451 FU-1: the Fake side has no key input. It checks that the
/// `tab-select` intent and snapshot the fixture's keyboard step expects are
/// what the contract yields for that selection; the GPUI side derives the
/// same index from the key press (`consumer_observation.rs`).
#[test]
fn ui_dvr_03_parity_tab_keyboard_intents() {
    let fx = selection_fixture();
    let kb = &fx["expect"]["tabs"]["keyboard"];
    let mut host = new_host();
    host.mount(view_spec(&fx)).expect("mount");
    host.drain_events();

    let index = usize::try_from(kb["selectedAfterKey"].as_u64().expect("keyboard target"))
        .expect("tab index");
    host.select_tab(host.generation(), &key("mixer_tabs"), index)
        .expect("select tab");
    assert_eq!(action_summaries(&mut host), kb["intents"]);
    let snap = host.observe(&key("mixer_tabs")).expect("tabs snapshot");
    assert_eq!(
        snap.selected.and_then(|r| u64::try_from(r).ok()),
        kb["selectedAfterKey"].as_u64()
    );
}

/// Issue #479: the shared keyboard-ops fixture — Fader/Seek/Knob/Toggle
/// key steps that both drivers must turn into the same intents and
/// committed values.
fn key_ops_fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/driver-lab/parity/key-ops.json");
    let text = std::fs::read_to_string(&path).expect("read key-ops fixture");
    serde_json::from_str(&text).expect("parse key-ops fixture")
}

/// The `ControlInput` a fixture key step declares — what the GPUI key
/// mapping must produce, dispatched here with `source: keyboard`.
fn keyboard_input(step: &Value) -> ControlInput {
    let source = Some(InputSource::from(InputSource::KEYBOARD));
    if let Some(adjust) = step["input"].get("adjust") {
        return ControlInput::Adjust {
            steps: adjust["steps"].as_f64().expect("adjust steps"),
            gesture_id: None,
            source,
            modifiers: Some(InputModifiers {
                fine: adjust["fine"].as_bool().unwrap_or(false),
            }),
            seq: None,
        };
    }
    let extreme = match step["input"]["extreme"].as_str().expect("extreme") {
        "min" => Extreme::Min,
        "max" => Extreme::Max,
        other => panic!("unknown extreme {other}"),
    };
    ControlInput::Set {
        value: ValueInput::Extreme(extreme),
        gesture_id: None,
        source,
        modifiers: Some(InputModifiers { fine: false }),
        seq: None,
    }
}

/// Issue #479: the Fake side has no key input, so it replays each
/// fixture step's declared `ControlInput` with `source: keyboard` and
/// checks the contract's answer: every gesture event carries
/// `source: "keyboard"`, mutating steps are `atomic: true` begin/commit
/// pairs while `disabled`/`read_only` controls report `unavailable`
/// instead, the minted intent kinds match `expect.*.intents`, and the
/// committed normalized lands on the fixture's `committedNormalized`.
/// The GPUI side presses the real key names (`consumer_observation.rs`).
#[test]
fn ui_dvr_03_parity_key_ops_keyboard_intents() {
    let fx = key_ops_fixture();
    let mut host = new_host();
    host.mount(view_spec(&fx)).expect("mount");
    host.drain_events();

    for (name, spec) in fx["expect"]["controls"]
        .as_object()
        .expect("controls object")
    {
        let k = key(name);
        let snap = host.observe(&k).expect("control snapshot");
        let committed = snap
            .committed
            .map(|c| c.normalized)
            .expect("committed value");
        let want_initial = spec["initialNormalized"].as_f64().expect("initial");
        assert!(
            (committed - want_initial).abs() < 1e-9,
            "{name} initial {committed} != {want_initial}"
        );

        let keys = spec["keys"].as_array().expect("keys");
        let mut kinds = Vec::new();
        let mut unavailable = 0usize;
        for step in keys {
            host.dispatch_input(host.generation(), &k, keyboard_input(step))
                .expect("dispatch key input");
            for event in host.drain_events() {
                match event {
                    ViewEvent::Gesture { event, .. } => {
                        assert_eq!(
                            event.meta().source.as_ref().map(|s| s.0.as_str()),
                            Some(InputSource::KEYBOARD),
                            "{name} gesture source"
                        );
                        match event {
                            // An unavailable control reports the press
                            // instead of mutating: the notification is
                            // not itself an atomic begin/commit pair.
                            InteractionEvent::Unavailable { reason, .. } => {
                                unavailable += 1;
                                assert_eq!(
                                    serde_json::to_value(reason).expect("reason json"),
                                    spec["unavailable"],
                                    "{name} unavailable reason"
                                );
                            }
                            _ => assert!(event.meta().atomic, "{name} atomic pair"),
                        }
                    }
                    ViewEvent::Intent { intent, .. } => kinds.push(
                        serde_json::to_value(&intent).expect("intent json")["kind"]
                            .as_str()
                            .expect("kind")
                            .to_string(),
                    ),
                    _ => {}
                }
            }
        }
        assert_eq!(kinds, strings(&spec["intents"]), "{name} intent kinds");
        let want_unavailable = if spec["unavailable"].is_string() {
            keys.len()
        } else {
            0
        };
        assert_eq!(
            unavailable, want_unavailable,
            "{name} unavailable notifications"
        );

        let last = keys.last().expect("last step");
        let want = last["committedNormalized"].as_f64().expect("committed");
        let committed = host
            .observe(&k)
            .and_then(|s| s.committed)
            .map(|c| c.normalized)
            .expect("committed after keys");
        assert!(
            (committed - want).abs() < 1e-9,
            "{name} committed {committed} != {want}"
        );
    }
}

/// Issue #314 slice 6: the AudioLens-shaped semantic oracle fixture —
/// the headless layer retains the declared Waveform/TimeAxis/Seek
/// payload verbatim (f64 values un-reinterpreted, gap/partial bins and
/// out-of-viewport ticks kept) and mounts every key. The drawn-plan
/// half is checked by the GPUI side (`consumer_observation.rs`).
#[test]
fn parity_viz_semantic_audiolens_payload_retained_fake() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/driver-lab/parity/viz-semantic-audiolens.json");
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
        .expect("parse fixture");
    let spec = view_spec(&fx);
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("fixture root must be a panel");
    };

    let wave = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Waveform(w) => Some(w),
            _ => None,
        })
        .expect("waveform");
    let want_wave = &fx["expect"]["waveform"];
    assert_eq!(wave.channels, 2);
    assert_eq!(wave.frames, 5_000_002_000);
    let declared: usize = wave.channel_bins.iter().map(Vec::len).sum();
    assert_eq!(
        u64::try_from(declared).expect("small"),
        want_wave["declaredBins"].as_u64().expect("declaredBins")
    );
    assert_eq!(
        wave.channel_bins
            .iter()
            .flatten()
            .filter(|b| b.count == 0)
            .count(),
        usize::try_from(want_wave["gapBins"].as_u64().expect("gapBins")).expect("small"),
        "gap bins pass through the headless payload"
    );
    // f64 source truth is carried bit-exact: no re-quantization, no
    // interpolation, partial bins keep their declared count. The
    // fixture decimals are chosen so serde_json and `str::parse`
    // agree bit-for-bit (serde_json 1.0.154 mis-rounds some 17-digit
    // decimals by 1 ulp — measured); the check pins that the mounted
    // spec keeps the parsed f64, e.g. never narrows to f32.
    let precision = &wave.channel_bins[0][3];
    assert_eq!(
        precision.min.to_bits(),
        (-0.123_456_789_012_345_6f64).to_bits()
    );
    assert_eq!(
        precision.max.to_bits(),
        0.987_654_321_098_765_4f64.to_bits()
    );
    assert_eq!(wave.channel_bins[0][1].count, 173, "partial bin kept");
    for (channel, want_channel) in want_wave["drawnChannels"]
        .as_array()
        .expect("channels")
        .iter()
        .enumerate()
    {
        let want_bins: Vec<(u64, u64)> = want_channel["bins"]
            .as_array()
            .expect("bins")
            .iter()
            .map(|b| (b[0].as_u64().expect("start"), b[1].as_u64().expect("end")))
            .collect();
        let drawn: Vec<(u64, u64)> = wave.channel_bins[channel]
            .iter()
            .filter(|b| b.count != 0 && b.min.is_finite() && b.max.is_finite())
            .map(|b| (b.start, b.end))
            .collect();
        assert_eq!(drawn, want_bins, "channel {channel} painted bins");
    }

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
        axis.ticks.iter().any(|t| t.at < 4_999_999_000.0)
            && axis.ticks.iter().any(|t| t.at > 5_000_000_000.0),
        "out-of-viewport ticks are retained in the payload"
    );

    // The Seek control keeps its domain value: committed plain is the
    // consumer-domain position, normalized is the [0, 1] fraction.
    let mut host = new_host();
    let ops = host.mount(spec).expect("mount");
    for k in strings(&fx["expect"]["mountedKeys"]) {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(&k) }), "{k}");
    }
    let snap = host.observe(&key("seek")).expect("seek snapshot");
    let committed = snap.committed.expect("committed value");
    let want_seek = &fx["expect"]["seek"];
    assert_eq!(
        committed.plain,
        want_seek["committedPlain"].as_f64().unwrap()
    );
    assert_eq!(
        committed.normalized,
        want_seek["committedNormalized"].as_f64().unwrap()
    );
}

/// Issue #314 slice 6: the Synvelis-shaped semantic oracle fixture —
/// RT-snapshot Meter bins (gap/partial/over-range/clamped viewport),
/// Spectrogram columns (gap/clamp/cull), and `TimeAxis` ticks are
/// retained verbatim by the headless payload.
#[test]
fn parity_viz_semantic_synvelis_payload_retained_fake() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/driver-lab/parity/viz-semantic-synvelis.json");
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
        .expect("parse fixture");
    let spec = view_spec(&fx);
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("fixture root must be a panel");
    };

    let meter = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Meter(m) => Some(m),
            _ => None,
        })
        .expect("meter");
    let want_meter = &fx["expect"]["meter"];
    let declared: usize = meter.channel_levels.iter().map(Vec::len).sum();
    assert_eq!(
        u64::try_from(declared).expect("small"),
        want_meter["declaredBins"].as_u64().expect("declaredBins")
    );
    assert_eq!(
        meter
            .channel_levels
            .iter()
            .flatten()
            .filter(|b| b.count == 0)
            .count(),
        usize::try_from(want_meter["gapBins"].as_u64().expect("gapBins")).expect("small")
    );
    // Over-range RT levels and sparse partial bins are payload truth —
    // clamping is a renderer concern, not a payload rewrite.
    let over = meter.channel_levels[0]
        .iter()
        .find(|b| b.peak > 1.0)
        .expect("over-range bin");
    assert_eq!(over.peak, 1.7);
    assert_eq!(over.rms, 1.2);
    assert!(
        meter.channel_levels[1]
            .iter()
            .any(|b| b.count == 37 && b.count < b.end - b.start),
        "sparse partial bin kept"
    );

    let spectrogram = panel
        .children
        .iter()
        .find_map(|c| match c {
            ComponentSpec::Spectrogram(s) => Some(s),
            _ => None,
        })
        .expect("spectrogram");
    let want_spec = &fx["expect"]["spectrogram"];
    assert_eq!(
        u64::try_from(spectrogram.columns.len()).expect("small"),
        want_spec["declaredColumns"]
            .as_u64()
            .expect("declaredColumns")
    );
    let cells: usize = spectrogram.columns.iter().map(|c| c.magnitudes.len()).sum();
    assert_eq!(
        u64::try_from(cells).expect("small"),
        want_spec["declaredCells"].as_u64().expect("declaredCells")
    );
    assert_eq!(
        spectrogram.columns.iter().filter(|c| c.count == 0).count(),
        usize::try_from(want_spec["gapColumns"].as_u64().expect("gapColumns")).expect("small")
    );
    // Magnitudes outside magnitude_range are payload truth — the
    // renderer clamps intensity, the payload keeps the source value.
    assert!(
        spectrogram
            .columns
            .iter()
            .flat_map(|c| &c.magnitudes)
            .any(|&m| m.to_bits() == (-70.0f64).to_bits() && m < spectrogram.magnitude_range.start)
            && spectrogram
                .columns
                .iter()
                .flat_map(|c| &c.magnitudes)
                .any(|&m| m.to_bits() == 10.0f64.to_bits() && m > spectrogram.magnitude_range.end),
        "out-of-range magnitudes retained verbatim"
    );

    let mut host = new_host();
    let ops = host.mount(spec).expect("mount");
    for k in strings(&fx["expect"]["mountedKeys"]) {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(&k) }), "{k}");
    }
}

fn wait_terminal(rt: &mut UiRuntime) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut all = Vec::new();
    while rt.has_running_jobs() && Instant::now() < deadline {
        all.extend(rt.poll(Some(Duration::from_millis(50))));
    }
    all.extend(rt.drain_jobs());
    all
}

/// Feeds runtime result events into the view as receipts — the same
/// `CommandReceipt::from_result` decoding the GPUI inbox applies.
fn deliver(host: &mut ViewHost, events: &[Event]) {
    for receipt in events.iter().filter_map(CommandReceipt::from_result) {
        host.apply_receipt(&receipt);
    }
}

/// Issue #456: a job-spawning command's receipts, produced by the runtime
/// under the command id the view minted, walk `pending` → `accepted`
/// (ack) → `applied` (job finished) or `rejected` (job panicked).
#[test]
fn ui_dvr_03_parity_job_command_receipt_phases() {
    let fx = selection_fixture();
    let want = &fx["expect"]["button"];
    let run = |op: &str, params: Value| -> Vec<Value> {
        let mut host = new_host();
        host.mount(view_spec(&fx)).expect("mount");
        host.drain_events();
        host.invoke(host.generation(), &key("run"))
            .expect("invoke button");
        let command_id = host
            .binding("run")
            .and_then(|b| b.state().pending_command_id)
            .expect("pending command id");
        let phase = |host: &ViewHost| {
            serde_json::to_value(host.observe(&key("run")).expect("button snapshot").phase)
                .expect("phase json")
        };
        let mut rt = UiRuntime::new(DataRegistry::new());
        rt.register_service(DemoService::new())
            .expect("demo service");
        let mut phases = vec![phase(&host)];
        deliver(&mut host, &rt.dispatch_op(command_id, op, params));
        phases.push(phase(&host));
        deliver(&mut host, &wait_terminal(&mut rt));
        phases.push(phase(&host));
        phases
    };
    let ack = want["jobReceipt"]["ack"].clone();
    assert_eq!(
        run("demo.sleep", json!({ "ms": 10 })),
        [
            want["phase"].clone(),
            ack.clone(),
            want["jobReceipt"]["applied"].clone()
        ]
    );
    assert_eq!(
        run("demo.panic", json!({})),
        [
            want["phase"].clone(),
            ack,
            want["jobReceipt"]["rejected"].clone()
        ]
    );
}
