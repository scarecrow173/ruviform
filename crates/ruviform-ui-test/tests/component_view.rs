//! Issue #387 semantic fixtures: driver-neutral Component/View contract
//! exercised on the fake (headless) layer.
//!
//! Coverage map (Driver Lab manifest):
//! - `UI-DVR-02` — `ViewHost` mount/reconcile/unmount, stable-key
//!   identity, stale-generation rejection.
//! - `UI-DVR-03` — `Begin → Preview* → Commit|Cancel` semantics incl.
//!   delayed/stale arrival, availability, atomic inputs.
//! - `LAB-SV-FADER-DOCK` — Synvelis vertical slice: Panel > Tabs > 2
//!   Faders; dock move preserves `ViewInstanceId`.
//! - `LAB-AL-WAVEFORM-SEEK` — `AudioLens` vertical slice: Waveform + Seek;
//!   bounded `WaveformBin` payload, NaN/gap pass-through.
//! - 2-theme equivalence — the semantic machine is theme-free: same
//!   spec + input sequence yields identical `ViewEvent` streams for
//!   `synvelis-dark` and `audiolens-dark`.

use ruviform_ui::behavior::{
    AvailabilityFlags, CommandIntent, ControlInput, DetachPolicy, InputSource, InteractionEvent,
    NoopReason, RejectReason, Revision, StateProjection, UnavailableReason, ValueDescriptor,
    ValueInput,
};
use ruviform_ui::dock::{DockTarget, DockWorkspace, DropZone, ViewInstanceId};
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpoch, ComponentKey, ComponentSpec, ComponentVisualState,
    ControlSpec, Orientation, ProjectionOp, TabSpec, TabsSpec, ViewError, ViewEvent, ViewHost,
    ViewSpec, WaveformBin, WaveformSpec,
};
use ruviform_ui::window::WindowId;
use ruviform_ui_theme::{ScaleContext, ThemeResolver, TokenDocument};

// ---------------------------------------------------------------------
// spec builders
// ---------------------------------------------------------------------

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn a11y_slider(label: &str) -> A11ySpec {
    A11ySpec {
        role: Some(A11yRole::Slider),
        label: Some(label.to_string()),
        orientation: Some(Orientation::Vertical),
        value_min: Some(0.0),
        value_max: Some(1.0),
        value_text: None,
    }
}

fn fader(name: &str, label: &str) -> ComponentSpec {
    ComponentSpec::Fader(ControlSpec {
        key: key(name),
        control_id: None,
        target: Some(format!("mixer.{name}")),
        descriptor: ValueDescriptor {
            min: 0.0,
            max: 1.0,
            default_value: Some(0.5),
            step: Some(0.05),
            ..ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: Orientation::Vertical,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: a11y_slider(label),
    })
}

fn seek(name: &str) -> ComponentSpec {
    let ComponentSpec::Fader(mut spec) = fader(name, "Seek") else {
        unreachable!()
    };
    spec.orientation = Orientation::Horizontal;
    spec.target = Some("transport.seek".to_string());
    ComponentSpec::Seek(spec)
}

fn wave_bin(start: u64, end: u64, min: f64, max: f64) -> WaveformBin {
    WaveformBin {
        start,
        end,
        min,
        max,
        rms: min.midpoint(max),
        count: (end - start).max(1),
    }
}

fn waveform(name: &str, bins: Vec<WaveformBin>) -> ComponentSpec {
    ComponentSpec::Waveform(WaveformSpec {
        key: key(name),
        sample_rate: 48_000,
        channels: 1,
        frames: 96_000,
        viewport: 0..96_000,
        channel_bins: vec![bins],
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Img),
            label: Some("Waveform".to_string()),
            ..A11ySpec::default()
        },
    })
}

fn panel(children: Vec<ComponentSpec>) -> ComponentSpec {
    ComponentSpec::Panel(ruviform_ui::view::PanelSpec {
        key: key("panel"),
        title: None,
        children,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

fn tabs(name: &str, tabs: Vec<(&str, ComponentSpec)>) -> ComponentSpec {
    ComponentSpec::Tabs(TabsSpec {
        key: key(name),
        active: 0,
        tabs: tabs
            .into_iter()
            .map(|(title, child)| TabSpec {
                key: key(title),
                title: title.to_string(),
                child: Box::new(child),
            })
            .collect(),
        selection: None,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::TabList),
            ..A11ySpec::default()
        },
    })
}

fn fader_view() -> ViewSpec {
    ViewSpec {
        root: panel(vec![fader("fader-ch1", "Ch 1"), fader("fader-ch2", "Ch 2")]),
    }
}

fn wave_view() -> ViewSpec {
    ViewSpec {
        root: panel(vec![
            waveform(
                "wave",
                vec![
                    wave_bin(0, 1000, -0.8, 0.9),
                    wave_bin(1000, 2000, -0.3, 0.4),
                ],
            ),
            seek("seek"),
        ]),
    }
}

fn host() -> ViewHost {
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    let ops = host.mount(fader_view()).expect("mount");
    assert_eq!(ops.len(), 3);
    host.drain_events();
    host
}

fn begin(id: &str) -> ControlInput {
    ControlInput::Begin {
        gesture_id: id.to_string(),
        source: InputSource::from("pointer"),
        origin_view: None,
        target: None,
        at: None,
        modifiers: None,
        seq: None,
    }
}

fn preview(id: &str, plain: f64) -> ControlInput {
    ControlInput::Preview {
        gesture_id: id.to_string(),
        value: ValueInput::Plain(plain),
        modifiers: None,
        seq: None,
    }
}

fn commit(id: &str) -> ControlInput {
    ControlInput::Commit {
        gesture_id: id.to_string(),
        value: None,
        modifiers: None,
        seq: None,
    }
}

fn cancel(id: &str) -> ControlInput {
    ControlInput::Cancel {
        gesture_id: id.to_string(),
        reason: None,
        seq: None,
    }
}

fn events_of(host: &mut ViewHost) -> Vec<ViewEvent> {
    host.drain_events()
}

fn intents(host: &mut ViewHost) -> Vec<CommandIntent> {
    events_of(host)
        .into_iter()
        .filter_map(|e| match e {
            ViewEvent::Intent { intent, .. } => Some(intent),
            _ => None,
        })
        .collect()
}

fn gesture_events(host: &mut ViewHost) -> Vec<InteractionEvent> {
    events_of(host)
        .into_iter()
        .filter_map(|e| match e {
            ViewEvent::Gesture { event, .. } => Some(event),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// UI-DVR-02 mount/reconcile/unmount
// ---------------------------------------------------------------------

#[test]
fn ui_dvr_02_mount_reconcile_unmount() {
    let mut host = host();
    let generation = host.generation();

    // Same key, changed props → Updated, committed/gesture preserved.
    let mut updated = fader_view();
    if let ComponentSpec::Panel(p) = &mut updated.root
        && let ComponentSpec::Fader(c) = &mut p.children[0]
    {
        c.a11y.label = Some("Ch 1 (renamed)".to_string());
    }
    let ops = host.update(updated, generation).expect("update");
    assert!(ops.contains(&ProjectionOp::Updated {
        key: key("fader-ch1")
    }));

    // Gesture survives a props-only update.
    let generation = host.generation();
    host.dispatch_input(generation, &key("fader-ch1"), begin("g1"))
        .expect("begin");
    host.drain_events();
    let mut updated = fader_view();
    if let ComponentSpec::Panel(p) = &mut updated.root
        && let ComponentSpec::Fader(c) = &mut p.children[1]
    {
        c.visual.error = Some("clipping".to_string());
    }
    let generation = host.generation();
    host.update(updated, generation)
        .expect("update while active");
    let snap = host.observe(&key("fader-ch1")).expect("snapshot");
    assert!(snap.display.is_some());

    // Removed key → Removed op + that key's active gesture cancelled;
    // sibling gestures are undisturbed.
    let generation = host.generation();
    host.dispatch_input(generation, &key("fader-ch2"), begin("g2"))
        .expect("begin on doomed key");
    host.drain_events();
    let mut removed = fader_view();
    if let ComponentSpec::Panel(p) = &mut removed.root {
        p.children.pop();
    }
    let generation = host.generation();
    let ops = host.update(removed, generation).expect("remove child");
    assert!(ops.contains(&ProjectionOp::Removed {
        key: key("fader-ch2")
    }));
    let g = gesture_events(&mut host);
    assert!(g.iter().any(
        |e| matches!(e, InteractionEvent::Cancel { meta, .. } if meta.control_id == "fader-ch2")
    ));
    // ch1's gesture is still alive — it can commit after the removal.
    let generation = host.generation();
    host.dispatch_input(generation, &key("fader-ch1"), commit("g1"))
        .expect("survivor commits");

    // detach() mints a new epoch — inputs minted under the old
    // generation are rejected without mutating.
    let dead = host.generation();
    let alive = host.detach("test-epoch");
    assert!(alive > dead);
    assert_eq!(
        host.update(fader_view(), dead),
        Err(ViewError::StaleGeneration {
            expected: alive,
            got: dead
        })
    );
    assert_eq!(
        host.dispatch_input(dead, &key("fader-ch1"), begin("g3")),
        Err(ViewError::StaleGeneration {
            expected: alive,
            got: dead
        })
    );
    host.attach();
    // The view is still usable under the new epoch.
    let generation = host.generation();
    host.dispatch_input(generation, &key("fader-ch1"), begin("g4"))
        .expect("post-attach begin");

    // Unmount tears everything down.
    let ops = host.unmount();
    assert!(ops.contains(&ProjectionOp::Unmounted { key: key("panel") }));
    assert_eq!(host.mounted_keys(), [] as [ComponentKey; 0]);
}

// ---------------------------------------------------------------------
// UI-DVR-03 gesture state machine
// ---------------------------------------------------------------------

#[test]
fn ui_dvr_03_begin_preview_commit() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(generation, &k, preview("g1", 0.7))
        .unwrap();
    host.dispatch_input(generation, &k, commit("g1")).unwrap();

    let intents = intents(&mut host);
    assert_eq!(intents.len(), 3);
    assert!(matches!(intents[0], CommandIntent::Begin { .. }));
    assert!(matches!(intents[1], CommandIntent::Preview { .. }));
    assert!(matches!(intents[2], CommandIntent::Commit { .. }));

    let snap = host.observe(&k).expect("snapshot");
    // 0.7 snaps to the 0.05 step grid; compare with tolerance.
    assert!((snap.committed.map(|v| v.plain).unwrap() - 0.7).abs() < 1e-9);
    assert!((snap.display.map(|v| v.plain).unwrap() - 0.7).abs() < 1e-9);
}

#[test]
fn ui_dvr_03_stale_and_concurrent_gestures() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(generation, &k, commit("g1")).unwrap();
    host.drain_events();

    // Delayed arrival after terminal: stale.
    let ev = host
        .dispatch_input(generation, &k, preview("g1", 0.2))
        .unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Rejected {
            reason: RejectReason::StaleGesture,
            ..
        }
    )));
    let ev = host.dispatch_input(generation, &k, begin("g1")).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Rejected {
            reason: RejectReason::AlreadyTerminated,
            ..
        }
    )));

    // Concurrent begin while another gesture is active: rejected, the
    // first gesture keeps running.
    host.dispatch_input(generation, &k, begin("g2")).unwrap();
    let ev = host.dispatch_input(generation, &k, begin("g3")).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Rejected {
            reason: RejectReason::GestureAlreadyActive,
            ..
        }
    )));
    host.dispatch_input(generation, &k, commit("g2")).unwrap();
    let snap = host.observe(&k).expect("snapshot");
    assert!(snap.committed.is_some());
}

#[test]
fn ui_dvr_03_unknown_and_noop_inputs() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    // Unknown gesture ids on non-begin inputs are a noop.
    for input in [preview("ghost", 0.1), commit("ghost"), cancel("ghost")] {
        let ev = host.dispatch_input(generation, &k, input).unwrap();
        assert!(ev.iter().any(|e| matches!(
            e,
            InteractionEvent::Noop {
                reason: NoopReason::NoActiveGesture,
                ..
            }
        )));
    }

    // Cancel terminates without touching committed value.
    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(generation, &k, preview("g1", 0.9))
        .unwrap();
    host.dispatch_input(generation, &k, cancel("g1")).unwrap();
    let snap = host.observe(&k).expect("snapshot");
    assert_eq!(snap.committed.map(|v| v.plain), Some(0.5));
}

#[test]
fn ui_dvr_03_detach_focus_lost_and_availability() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    // Detach mid-gesture cancels (default policy, reason `unmount`).
    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(
        generation,
        &k,
        ControlInput::Detach {
            gesture_id: None,
            policy: None,
            reason: None,
            seq: None,
        },
    )
    .unwrap();
    let g = gesture_events(&mut host);
    assert!(g.iter().any(|e| matches!(e, InteractionEvent::Cancel {
        reason, ..
    } if reason.0 == "unmount")));

    // focus-lost cancels with `lost-focus`.
    host.dispatch_input(generation, &k, begin("g2")).unwrap();
    host.dispatch_input(
        generation,
        &k,
        ControlInput::FocusLost {
            gesture_id: None,
            seq: None,
        },
    )
    .unwrap();
    let g = gesture_events(&mut host);
    assert!(g.iter().any(|e| matches!(e, InteractionEvent::Cancel {
        reason, ..
    } if reason.0 == "lost-focus")));

    // Availability flip mid-gesture cancels then reports unavailable.
    host.dispatch_input(generation, &k, begin("g3")).unwrap();
    let ev = host
        .set_availability(
            &k,
            AvailabilityFlags {
                disabled: true,
                ..AvailabilityFlags::default()
            },
        )
        .unwrap();
    assert!(ev.iter().any(|e| matches!(e, InteractionEvent::Cancel {
        reason, ..
    } if reason.0 == "explicit")));
    let ev = host.dispatch_input(generation, &k, begin("g4")).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Unavailable {
            reason: UnavailableReason::Disabled,
            ..
        }
    )));
}

#[test]
fn ui_dvr_03_invalid_and_atomic_inputs() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    // Non-finite input rejected, gesture undisturbed.
    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    let ev = host
        .dispatch_input(generation, &k, preview("g1", f64::NAN))
        .unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Rejected {
            reason: RejectReason::NonFiniteInput,
            ..
        }
    )));
    // Invalid commit leaves the gesture active for retry.
    let ev = host
        .dispatch_input(
            generation,
            &k,
            ControlInput::Commit {
                gesture_id: "g1".to_string(),
                value: Some(ValueInput::Plain(f64::INFINITY)),
                modifiers: None,
                seq: None,
            },
        )
        .unwrap();
    assert!(
        ev.iter()
            .any(|e| matches!(e, InteractionEvent::Rejected { .. }))
    );
    host.dispatch_input(generation, &k, commit("g1")).unwrap();
    host.drain_events();

    // Atomic set (no gesture id): begin+commit pair flagged atomic.
    host.dispatch_input(
        generation,
        &k,
        ControlInput::Set {
            value: ValueInput::Plain(0.25),
            gesture_id: None,
            source: None,
            modifiers: None,
            seq: None,
        },
    )
    .unwrap();
    let intents = intents(&mut host);
    assert!(
        intents
            .iter()
            .any(|i| matches!(i, CommandIntent::Begin { atomic: true, .. }))
    );
    assert!(intents.iter().any(|i| matches!(i, CommandIntent::Commit {
        atomic: true, value, ..
    } if (value.plain - 0.25).abs() < 1e-9)));

    // Reset jumps to the declared default.
    host.dispatch_input(
        generation,
        &k,
        ControlInput::Reset {
            gesture_id: None,
            source: None,
            seq: None,
        },
    )
    .unwrap();
    let snap = host.observe(&k).expect("snapshot");
    assert_eq!(snap.committed.map(|v| v.plain), Some(0.5));
}

#[test]
fn ui_dvr_03_sequence_dedup() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    let seq = |n: u64| Some(n);
    let begin_seq = |n: u64| ControlInput::Begin {
        gesture_id: "g1".to_string(),
        source: InputSource::from("pointer"),
        origin_view: None,
        target: None,
        at: None,
        modifiers: None,
        seq: seq(n),
    };
    let preview_seq = |n: u64| ControlInput::Preview {
        gesture_id: "g1".to_string(),
        value: ValueInput::Plain(0.3),
        modifiers: None,
        seq: seq(n),
    };

    host.dispatch_input(generation, &k, begin_seq(1)).unwrap();
    host.dispatch_input(generation, &k, preview_seq(2)).unwrap();
    // Duplicate seq → noop; lower seq → rejected.
    let ev = host.dispatch_input(generation, &k, preview_seq(2)).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Noop {
            reason: NoopReason::DuplicateInput,
            ..
        }
    )));
    let ev = host.dispatch_input(generation, &k, preview_seq(1)).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        InteractionEvent::Rejected {
            reason: RejectReason::OutOfOrder,
            ..
        }
    )));
}

// ---------------------------------------------------------------------
// LAB-SV-FADER-DOCK (fake layer)
// ---------------------------------------------------------------------

#[test]
fn lab_sv_fader_dock() {
    // Synvelis slice: Panel > Tabs(ch1: Fader, ch2: Fader).
    let view_id = ViewInstanceId::new(42).expect("nonzero");
    let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
    let spec = ViewSpec {
        root: panel(vec![tabs(
            "mixer-tabs",
            vec![
                ("Ch 1", fader("fader-ch1", "Ch 1")),
                ("Ch 2", fader("fader-ch2", "Ch 2")),
            ],
        )]),
    };
    let ops = host.mount(spec).expect("mount");
    for k in ["panel", "mixer-tabs", "fader-ch1", "fader-ch2"] {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(k) }), "{k}");
    }

    // Dock the view in a workspace window; a move across leaves keeps
    // the same ViewInstanceId (dock-tree contract is id-keyed).
    let mut ws = DockWorkspace::new();
    let w1 = WindowId::new(1).expect("nonzero");
    let w2 = WindowId::new(2).expect("nonzero");
    ws.register_window(w1);
    ws.register_window(w2);
    ws.attach_view(view_id, DockTarget::Window(w1)).unwrap();
    let other = ViewInstanceId::new(43).expect("nonzero");
    ws.attach_view(other, DockTarget::Window(w2)).unwrap();

    let token = ws
        .begin_move(
            view_id,
            DockTarget::Zone {
                window: w2,
                anchor: other,
                zone: DropZone::Tab,
            },
        )
        .unwrap();
    ws.commit_move(token).unwrap();
    assert_eq!(ws.owner_of(view_id), Some(w2));
    // The host sees the move as detach+attach; detach mints a new
    // generation epoch and stale ids die with the old one.
    let detached_gen = host.detach("dock-move");
    host.attach();
    assert_eq!(host.generation(), detached_gen);

    // Gesture on fader-ch2 works identically after the move.
    let generation = host.generation();
    let k = key("fader-ch2");
    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(generation, &k, preview("g1", 0.8))
        .unwrap();
    host.dispatch_input(generation, &k, commit("g1")).unwrap();
    let intents = intents(&mut host);
    assert!(intents.iter().any(|i| matches!(i, CommandIntent::Commit {
        target, value, ..
    } if target == "mixer.fader-ch2" && (value.plain - 0.8).abs() < 1e-9)));

    // Snapshot exposes committed + display consistently.
    let snap = host.observe(&k).expect("snapshot");
    assert_eq!(snap.committed.map(|v| v.plain), Some(0.8));
}

// ---------------------------------------------------------------------
// LAB-AL-WAVEFORM-SEEK (fake layer)
// ---------------------------------------------------------------------

#[test]
fn lab_al_waveform_seek() {
    let mut host = ViewHost::new(
        ViewInstanceId::new(7).expect("nonzero"),
        AttachEpoch::new(1),
    );
    let ops = host.mount(wave_view()).expect("mount");
    for k in ["panel", "wave", "seek"] {
        assert!(ops.contains(&ProjectionOp::Mounted { key: key(k) }), "{k}");
    }

    // Seek gesture → transport.seek intents.
    let generation = host.generation();
    let k = key("seek");
    host.dispatch_input(generation, &k, begin("s1")).unwrap();
    host.dispatch_input(generation, &k, preview("s1", 0.5))
        .unwrap();
    host.dispatch_input(generation, &k, commit("s1")).unwrap();
    let intents = intents(&mut host);
    assert!(intents.iter().any(|i| matches!(i, CommandIntent::Commit {
        target, ..
    } if target == "transport.seek")));

    // Waveform accepts payload-only updates: a new LOD resolution at a
    // different "scale" (fewer bins covering the same viewport) mounts
    // an Updated op and keeps the key.
    let updated = ViewSpec {
        root: panel(vec![
            waveform("wave", vec![wave_bin(0, 2000, -0.8, 0.9)]),
            seek("seek"),
        ]),
    };
    let generation = host.generation();
    let ops = host.update(updated, generation).expect("re-resolve bins");
    assert!(ops.contains(&ProjectionOp::Updated { key: key("wave") }));
    // Committed seek value survives a sibling payload update.
    let snap = host.observe(&key("seek")).expect("snapshot");
    assert_eq!(snap.committed.map(|v| v.plain), Some(0.5));

    // NaN/gap bins are payload data and pass through validation.
    let mut gapped = wave_view();
    if let ComponentSpec::Panel(p) = &mut gapped.root
        && let ComponentSpec::Waveform(w) = &mut p.children[0]
    {
        w.channel_bins[0].push(WaveformBin {
            start: 2000,
            end: 3000,
            min: f64::NAN,
            max: f64::NAN,
            rms: f64::NAN,
            count: 0,
        });
    }
    let generation = host.generation();
    host.update(gapped, generation).expect("gap payload ok");
}

#[test]
fn lab_al_waveform_bounded() {
    let mut host = ViewHost::new(
        ViewInstanceId::new(8).expect("nonzero"),
        AttachEpoch::new(1),
    );
    let bins: Vec<WaveformBin> = (0..100_000u64)
        .map(|i| wave_bin(i, i + 1, -0.5, 0.5))
        .collect();
    let spec = ViewSpec {
        root: waveform("wave", bins),
    };
    assert!(matches!(
        host.mount(spec),
        Err(ViewError::InvalidSpec { .. })
    ));
}

// ---------------------------------------------------------------------
// LAB story matrix (Issue #426): 2 theme × 2 scale × semantic state
// ---------------------------------------------------------------------

fn sv_fader_dock_view() -> ViewSpec {
    ViewSpec {
        root: panel(vec![tabs(
            "mixer-tabs",
            vec![
                ("Ch 1", fader("fader-ch1", "Ch 1")),
                ("Ch 2", fader("fader-ch2", "Ch 2")),
            ],
        )]),
    }
}

/// One story control walked through the seven story states, one trace
/// line per step: idle → focus (begin) → drag (preview) → commit →
/// stale (late preview after terminal) → invalid (NaN preview) →
/// teardown (unmount).
fn story_trace(host: &mut ViewHost, k: &ComponentKey, empty: ViewSpec) -> Vec<String> {
    let g = host.generation();
    let idle = host.observe(k).and_then(|s| s.committed).map(|v| v.plain);
    let mut trace = vec![format!("idle {idle:?}")];
    let focus = host.dispatch_input(g, k, begin("s1")).unwrap();
    trace.push(format!("focus {focus:?}"));
    let drag = host.dispatch_input(g, k, preview("s1", 0.8)).unwrap();
    trace.push(format!("drag {drag:?}"));
    let committed = host.dispatch_input(g, k, commit("s1")).unwrap();
    trace.push(format!("commit {committed:?}"));
    trace.push(format!("commit-intents {:?}", intents(host)));
    let stale = host.dispatch_input(g, k, preview("s1", 0.2)).unwrap();
    trace.push(format!("stale {stale:?}"));
    host.dispatch_input(g, k, begin("s2")).unwrap();
    let invalid = host.dispatch_input(g, k, preview("s2", f64::NAN)).unwrap();
    trace.push(format!("invalid {invalid:?}"));
    let teardown = host.update(empty, g).unwrap();
    trace.push(format!("teardown {teardown:?}"));
    trace
}

/// Runs the story walk for every {theme, scale} combination and asserts
/// the semantic trace is identical. Each combination resolves its theme
/// at the scale under test; scale changes only the `ScaleContext` and
/// tokens stay identical. `ViewHost` holds neither theme nor scale, so
/// the trace must not depend on either axis.
fn assert_story_matrix(
    story: &str,
    view: fn() -> ViewSpec,
    empty: fn() -> ViewSpec,
    k: &ComponentKey,
) -> Vec<String> {
    let mut reference: Option<Vec<String>> = None;
    for theme_name in ["synvelis-dark", "audiolens-dark"] {
        let base = resolve_fixture(theme_name);
        for scale_factor in [1.0, 2.0] {
            let themed = base.with_scale(ScaleContext {
                scale_factor,
                rem_base_px: 16.0,
            });
            assert_eq!(themed.scale().scale_factor, scale_factor);
            assert_eq!(
                themed.tokens().collect::<Vec<_>>(),
                base.tokens().collect::<Vec<_>>(),
            );
            let mut host = ViewHost::new(
                ViewInstanceId::new(9).expect("nonzero"),
                AttachEpoch::new(1),
            );
            host.mount(view()).expect("mount");
            host.drain_events();
            let trace = story_trace(&mut host, k, empty());
            match &reference {
                Some(want) => {
                    assert_eq!(
                        &trace, want,
                        "{story} @ {theme_name} x scale {scale_factor}"
                    );
                }
                None => reference = Some(trace),
            }
        }
    }
    reference.expect("matrix ran")
}

fn assert_story_states(trace: &[String], expected: &[(&str, &str)]) {
    for (state, marker) in expected {
        let line = trace
            .iter()
            .find(|l| l.starts_with(&format!("{state} ")))
            .unwrap_or_else(|| panic!("no `{state}` step in {trace:?}"));
        assert!(
            line.contains(marker),
            "{state}: `{marker}` missing in {line}"
        );
    }
}

#[test]
fn lab_story_sv_fader_dock_matrix() {
    let trace = assert_story_matrix(
        "LAB-SV-FADER-DOCK",
        sv_fader_dock_view,
        || ViewSpec {
            root: panel(vec![]),
        },
        &key("fader-ch1"),
    );
    assert_story_states(
        &trace,
        &[
            ("idle", "Some(0.5)"),
            ("focus", "Begin"),
            ("drag", "Preview"),
            ("commit", "Commit"),
            ("stale", "StaleGesture"),
            ("invalid", "NonFiniteInput"),
            ("teardown", "Removed"),
        ],
    );
}

#[test]
fn lab_story_al_waveform_seek_matrix() {
    let trace = assert_story_matrix(
        "LAB-AL-WAVEFORM-SEEK",
        wave_view,
        || ViewSpec {
            root: panel(vec![waveform("wave", vec![wave_bin(0, 1000, -0.8, 0.9)])]),
        },
        &key("seek"),
    );
    assert_story_states(
        &trace,
        &[
            ("idle", "Some(0.5)"),
            ("focus", "Begin"),
            ("drag", "Preview"),
            ("commit", "Commit"),
            ("stale", "StaleGesture"),
            ("invalid", "NonFiniteInput"),
            ("teardown", "Removed"),
        ],
    );
}

// ---------------------------------------------------------------------
// 2-theme semantic equivalence
// ---------------------------------------------------------------------

fn resolve_fixture(name: &str) -> ruviform_ui_theme::ResolvedTheme {
    let path = format!(
        "{}/fixtures/themes/{name}.dtcg.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let doc = TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    ThemeResolver::resolve(&[doc], "dark").unwrap_or_else(|e| panic!("resolve {name}: {e}"))
}

#[test]
fn two_themes_semantic_equivalence() {
    let synvelis = resolve_fixture("synvelis-dark");
    let audiolens = resolve_fixture("audiolens-dark");
    // Fixtures actually differ (semantic palettes must not alias).
    assert_ne!(
        synvelis.color("accent.primary"),
        audiolens.color("accent.primary")
    );
    assert_ne!(
        synvelis.color("viz.waveform"),
        audiolens.color("viz.waveform")
    );

    // ViewHost is theme-free: identical spec + input sequence produces
    // an identical semantic stream regardless of which product theme
    // a renderer would pair it with.
    let run = || {
        let mut h = host();
        let generation = h.generation();
        let k = key("fader-ch1");
        h.dispatch_input(generation, &k, begin("g1")).unwrap();
        h.dispatch_input(generation, &k, preview("g1", 0.6))
            .unwrap();
        h.dispatch_input(generation, &k, commit("g1")).unwrap();
        let events = events_of(&mut h);
        let snap = h.observe(&k).map(|s| (s.committed, s.phase));
        (events, snap)
    };
    let (a_events, a_snap) = run();
    let (b_events, b_snap) = run();
    assert_eq!(a_events.len(), b_events.len());
    for (a, b) in a_events.iter().zip(b_events.iter()) {
        match (a, b) {
            (ViewEvent::Intent { intent: ai, .. }, ViewEvent::Intent { intent: bi, .. }) => {
                assert_eq!(ai, bi);
            }
            (ViewEvent::Op(ao), ViewEvent::Op(bo)) => assert_eq!(ao, bo),
            (ViewEvent::Gesture { event: ae, .. }, ViewEvent::Gesture { event: be, .. }) => {
                assert_eq!(ae, be);
            }
            _ => panic!("event streams diverged: {a:?} vs {b:?}"),
        }
    }
    assert_eq!(a_snap, b_snap);
}

// ---------------------------------------------------------------------
// LAB-SV-NATIVE-SLOT (fake layer — Issue #387 × #388 glue)
// ---------------------------------------------------------------------

/// A `NativeSlot` component participates in key-driven reconciliation
/// like every other leaf: `Mounted`/`Updated`/`Removed` ops, spec
/// validation of `min_size`, no interaction registration (it is not a
/// control — slot state flows through the driver-side binding map).
#[test]
fn lab_sv_native_slot_reconcile() {
    use ruviform_ui::view::NativeSlotSpec;

    let slot_component = |key_name: &str| {
        ComponentSpec::NativeSlot(NativeSlotSpec {
            key: key(key_name),
            min_size: Some([320.0, 240.0]),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    };
    let mut host = ViewHost::new(
        ViewInstanceId::new(9).expect("nonzero"),
        AttachEpoch::new(1),
    );
    let ops = host
        .mount(ViewSpec {
            root: panel(vec![slot_component("dock.editor")]),
        })
        .expect("mount");
    assert!(ops.contains(&ProjectionOp::Mounted {
        key: key("dock.editor")
    }));
    // Non-interactive: a mounted non-control component absorbs inputs
    // without events; unmounted keys still fail UnknownComponent.
    let generation = host.generation();
    let events = host
        .dispatch_input(generation, &key("dock.editor"), begin("x"))
        .expect("non-control absorbs");
    assert_eq!(events, [] as [InteractionEvent; 0]);
    let err = host
        .dispatch_input(generation, &key("unmounted"), begin("x"))
        .unwrap_err();
    assert!(matches!(err, ViewError::UnknownComponent { .. }));

    // Same key, changed props → Updated; key gone → Removed.
    let generation = host.generation();
    let mut updated = host.spec().expect("spec").clone();
    if let ComponentSpec::Panel(p) = &mut updated.root {
        p.children[0] = slot_component("dock.editor");
        if let ComponentSpec::NativeSlot(n) = &mut p.children[0] {
            n.min_size = Some([640.0, 480.0]);
        }
    }
    let ops = host.update(updated, generation).expect("update");
    assert!(ops.contains(&ProjectionOp::Updated {
        key: key("dock.editor")
    }));

    let generation = host.generation();
    let ops = host
        .update(
            ViewSpec {
                root: panel(vec![]),
            },
            generation,
        )
        .expect("remove");
    assert!(ops.contains(&ProjectionOp::Removed {
        key: key("dock.editor")
    }));

    // Validation: non-finite/negative min_size is an InvalidSpec.
    let mut bad = ViewSpec {
        root: panel(vec![slot_component("dock.editor")]),
    };
    if let ComponentSpec::Panel(p) = &mut bad.root
        && let ComponentSpec::NativeSlot(n) = &mut p.children[0]
    {
        n.min_size = Some([f64::NAN, 10.0]);
    }
    let err = host.mount(bad).unwrap_err();
    assert!(matches!(err, ViewError::InvalidSpec { .. }));
}

// ---------------------------------------------------------------------
// UI-DVR-09 authoritative projection → controlled display (Issue #414)
// ---------------------------------------------------------------------

fn projection(value: serde_json::Value, revision: u64) -> StateProjection<serde_json::Value> {
    StateProjection {
        revision: Revision::Num(revision),
        value,
        provenance: None,
    }
}

fn display_of(host: &ViewHost, name: &str) -> ruviform_ui::behavior::ControlValue {
    host.observe(&key(name))
        .and_then(|s| s.display)
        .expect("display")
}

/// An authoritative projection that lands while the control is idle
/// must update the value the renderer reads (`snapshot.display`),
/// not merely populate `observed` — and the write must mint no
/// gesture events or intents (the controlled-write loop stays
/// closed).
#[test]
fn ui_dvr_09_projection_updates_controlled_display() {
    let mut host = host();
    let k = key("fader-ch1");
    assert!((display_of(&host, "fader-ch1").plain - 0.5).abs() < 1e-9);

    assert!(host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.8}), 1)
    ));
    let snap = host.observe(&k).expect("snapshot");
    // The renderer-consumed value follows the observed truth.
    assert!((snap.display.expect("display").normalized - 0.8).abs() < 1e-9);
    assert!((snap.committed.expect("committed").normalized - 0.8).abs() < 1e-9);
    assert!(snap.observed.is_some());
    assert_eq!(snap.phase, ruviform_ui::behavior::CommandPhase::Observed);
    // Silent write: no Gesture/Intent events were minted.
    assert!(
        !events_of(&mut host)
            .iter()
            .any(|e| matches!(e, ViewEvent::Intent { .. } | ViewEvent::Gesture { .. }))
    );

    // A non-control-value payload is still recorded as observed but
    // must not be guessed into the display.
    assert!(host.apply_projection("fader-ch1", &projection(serde_json::json!("chorus"), 2)));
    assert!((display_of(&host, "fader-ch1").normalized - 0.8).abs() < 1e-9);

    // Stale/equal revisions never touch the display.
    assert!(!host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.1}), 1)
    ));
    assert!((display_of(&host, "fader-ch1").normalized - 0.8).abs() < 1e-9);
}

/// During an active gesture the authoritative write is staged, not
/// applied: the local preview keeps the display, and the staged
/// truth lands when the gesture terminates — on commit the last
/// observed state wins, on cancel the aborted preview is replaced
/// by the observed truth rather than the stale committed value.
#[test]
fn ui_dvr_09_projection_respects_gesture_preview() {
    let mut host = host();
    let generation = host.generation();
    let k = key("fader-ch1");

    // --- commit path ---
    host.dispatch_input(generation, &k, begin("g1")).unwrap();
    host.dispatch_input(generation, &k, preview("g1", 0.7))
        .unwrap();
    assert!(host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.3}), 1)
    ));
    // Preview wins mid-gesture; observed still records the truth.
    assert!((display_of(&host, "fader-ch1").normalized - 0.7).abs() < 1e-9);
    assert!(host.observe(&k).expect("snap").observed.is_some());
    host.dispatch_input(generation, &k, commit("g1")).unwrap();
    // Staged truth lands as the committed display.
    assert!((display_of(&host, "fader-ch1").normalized - 0.3).abs() < 1e-9);
    host.drain_events();

    // --- cancel path ---
    host.dispatch_input(generation, &k, begin("g2")).unwrap();
    host.dispatch_input(generation, &k, preview("g2", 0.9))
        .unwrap();
    assert!(host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.2}), 2)
    ));
    assert!((display_of(&host, "fader-ch1").normalized - 0.9).abs() < 1e-9);
    host.dispatch_input(generation, &k, cancel("g2")).unwrap();
    // Cancel restores nothing stale: the staged observed value lands.
    assert!((display_of(&host, "fader-ch1").normalized - 0.2).abs() < 1e-9);
}

/// Detach absorbs projections (binding goes `stale`); after reattach
/// the next advancing projection drives the display again.
#[test]
fn ui_dvr_09_projection_detach_reattach() {
    let mut host = host();
    assert!(host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.8}), 1)
    ));
    host.detach("dock-transfer");
    assert!(!host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.9}), 2)
    ));
    assert!((display_of(&host, "fader-ch1").normalized - 0.8).abs() < 1e-9);
    host.attach();
    assert!(host.apply_projection(
        "fader-ch1",
        &projection(serde_json::json!({"normalized": 0.95}), 3)
    ));
    assert!((display_of(&host, "fader-ch1").normalized - 0.95).abs() < 1e-9);
}
