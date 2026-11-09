//! Driver Lab story fixtures (Issue #426): the canonical `ViewSpec`s
//! behind `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK`, the shared
//! semantic-state walk every [`SemanticGuiTest`] adapter must
//! reproduce, and the single expected trace — the oracle the Fake and
//! GPUI-simulated layers assert against identically.
//!
//! The walk is adapter-neutral: it drives the `SemanticGuiTest` port
//! (`dispatch_input`/`settle`/`drain_intents`/`observe`) and records a
//! normalized [`StoryStep`] per story state. `dispatch_input` is
//! semantic dispatch — it bypasses hit-testing and is never pointer,
//! OS, draw-plan, or pixel evidence. Theme/scale enter only through
//! the resolved theme a mount supplies; the semantic machine itself
//! is theme- and scale-free, so the same trace must hold for every
//! {theme, scale} cell.
//!
//! The teardown step is the one adapter-specific operation (spec
//! replacement has no port method), so the caller supplies it as a
//! closure; everything else — dispatch order, settle points, intent
//! accounting, observation — is shared here.
//!
//! **RT contract:** non-RT only.

use ruviform_ui::behavior::{
    CommandIntent, CommandPhase, ControlInput, DetachPolicy, ValueDescriptor, ValueInput,
};
use ruviform_ui::view::{
    A11yRole, A11ySpec, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec,
    Orientation, PanelSpec, TabSpec, TabsSpec, ViewSpec, WaveformBin, WaveformSpec,
};

use crate::scenario::{ScenarioError, ScenarioTarget, SemanticGuiTest, begin, commit, preview};

// ---------------------------------------------------------------------
// canonical story specs — single source of truth for both layers
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
    ComponentSpec::Panel(PanelSpec {
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

/// One Driver Lab story: the manifest id, the theme `stories.json`
/// declares for it, the canonical mounted spec, the control the walk
/// drives, and the spec the teardown step mounts.
pub struct StoryFixture {
    /// Manifest story id (`LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK`).
    pub story_id: &'static str,
    /// Theme the story declares in `stories.json`.
    pub theme: &'static str,
    /// The mounted `ViewSpec` — identical on every adapter layer.
    pub spec: ViewSpec,
    /// Component the semantic walk drives.
    pub walk_key: ComponentKey,
    /// Spec the teardown step mounts (controls removed; payload kept
    /// where the story has one).
    pub teardown_spec: ViewSpec,
}

/// `LAB-SV-FADER-DOCK`: Panel > Tabs > 2×Fader — the Synvelis slice.
#[must_use]
pub fn sv_fader_dock_fixture() -> StoryFixture {
    StoryFixture {
        story_id: "LAB-SV-FADER-DOCK",
        theme: "synvelis-dark",
        spec: ViewSpec {
            root: panel(vec![tabs(
                "mixer-tabs",
                vec![
                    ("Ch 1", fader("fader-ch1", "Ch 1")),
                    ("Ch 2", fader("fader-ch2", "Ch 2")),
                ],
            )]),
        },
        walk_key: key("fader-ch1"),
        teardown_spec: ViewSpec {
            root: panel(vec![]),
        },
    }
}

/// `LAB-AL-WAVEFORM-SEEK`: Panel > Waveform + Seek — the `AudioLens`
/// slice.
#[must_use]
pub fn al_waveform_seek_fixture() -> StoryFixture {
    StoryFixture {
        story_id: "LAB-AL-WAVEFORM-SEEK",
        theme: "audiolens-dark",
        spec: ViewSpec {
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
        },
        walk_key: key("seek"),
        teardown_spec: ViewSpec {
            root: panel(vec![waveform("wave", vec![wave_bin(0, 1000, -0.8, 0.9)])]),
        },
    }
}

/// Every canonical story fixture — the set `stories.json` must
/// cover. Manifest/story drift checks compare these ids against the
/// manifest's story ids so a renamed fixture cannot put a
/// nonexistent story id into golden keys or evidence.
#[must_use]
pub fn all_fixtures() -> [StoryFixture; 2] {
    [sv_fader_dock_fixture(), al_waveform_seek_fixture()]
}

// ---------------------------------------------------------------------
// shared semantic walk + oracle
// ---------------------------------------------------------------------

/// One normalized observation of a story state — adapter-neutral
/// evidence. `committed`/`display` are plain-domain values;
/// `phase`/`observe_error` are `None`/absent only when the walked
/// component can no longer be observed (the teardown step).
#[derive(Debug, Clone, PartialEq)]
pub struct StoryStep {
    /// Story state name: `idle`/`focus`/`drag`/`commit`/`stale`/
    /// `invalid`/`teardown`.
    pub state: &'static str,
    /// `CommandIntent` kinds minted while the state ran, in order.
    pub intents: Vec<&'static str>,
    /// `snapshot.committed.plain` after the step.
    pub committed: Option<f64>,
    /// `snapshot.display.plain` after the step.
    pub display: Option<f64>,
    /// `snapshot.phase` after the step.
    pub phase: Option<CommandPhase>,
    /// Whether the walked component is still mounted.
    pub mounted: bool,
    /// Stable [`ScenarioError`] kind when `observe` failed.
    pub observe_error: Option<&'static str>,
}

/// The canonical expected trace for [`lab_story_walk`] — the single
/// oracle every adapter layer asserts against. Values are the
/// `ViewHost` semantics both adapters share; adapters must not
/// diverge from this without a contract reason.
#[must_use]
pub fn expected_lab_story_trace() -> Vec<StoryStep> {
    vec![
        StoryStep {
            state: "idle",
            intents: vec![],
            committed: Some(0.5),
            display: Some(0.5),
            phase: Some(CommandPhase::Idle),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "focus",
            intents: vec!["begin"],
            committed: Some(0.5),
            display: Some(0.5),
            phase: Some(CommandPhase::Preview),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "drag",
            intents: vec!["preview"],
            committed: Some(0.5),
            display: Some(0.8),
            phase: Some(CommandPhase::Preview),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "commit",
            intents: vec!["commit"],
            committed: Some(0.8),
            display: Some(0.8),
            phase: Some(CommandPhase::Pending),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "stale",
            intents: vec![],
            committed: Some(0.8),
            display: Some(0.8),
            phase: Some(CommandPhase::Pending),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "invalid",
            intents: vec!["begin"],
            committed: Some(0.8),
            display: Some(0.8),
            phase: Some(CommandPhase::Preview),
            mounted: true,
            observe_error: None,
        },
        StoryStep {
            state: "teardown",
            // The `invalid` step leaves gesture `s2` open; unmounting
            // the control cancels it per detach policy and the binding
            // mints a `cancel` intent before the key disappears.
            intents: vec!["cancel"],
            committed: None,
            display: None,
            phase: None,
            mounted: false,
            observe_error: Some("unknown_component"),
        },
    ]
}

/// Walks `key` through the seven story states on `target` and returns
/// the normalized trace. `unmount` performs the adapter-specific spec
/// replacement for the teardown step (Fake:
/// `FakeGuiTest::update_view`; GPUI:
/// `GpuiDriver::update_view` — both leave the host generation stable,
/// so the pinned target stays valid and `observe` reports
/// `UnknownComponent` for the removed key).
///
/// # Errors
/// Propagates the first [`ScenarioError`] from dispatch/settle/observe
/// — a walk that cannot run is a contract failure, not a verdict.
pub fn lab_story_walk<G: SemanticGuiTest>(
    gui: &mut G,
    target: &ScenarioTarget,
    key: &ComponentKey,
    unmount: impl FnOnce(&mut G) -> Result<(), ScenarioError>,
) -> Result<Vec<StoryStep>, ScenarioError> {
    let mut steps = Vec::with_capacity(7);

    // idle — mounted baseline, no dispatch.
    gui.settle()?;
    steps.push(record_step(gui, target, key, "idle"));

    // focus — Begin opens the gesture.
    dispatch_settled(gui, target, key, begin("s1"))?;
    steps.push(record_step(gui, target, key, "focus"));

    // drag — Preview carries the tentative value.
    dispatch_settled(gui, target, key, preview("s1", ValueInput::Plain(0.8)))?;
    steps.push(record_step(gui, target, key, "drag"));

    // commit — Commit promotes the previewed value.
    dispatch_settled(gui, target, key, commit("s1"))?;
    steps.push(record_step(gui, target, key, "commit"));

    // stale — a late preview for the terminated gesture is rejected
    // inside the host: no intent mints and committed/display hold.
    dispatch_settled(gui, target, key, preview("s1", ValueInput::Plain(0.2)))?;
    steps.push(record_step(gui, target, key, "stale"));

    // invalid — a fresh gesture begins (Begin mints), but the NaN
    // preview is rejected (NonFiniteInput): only `begin` lands.
    dispatch_settled(gui, target, key, begin("s2"))?;
    dispatch_settled(gui, target, key, preview("s2", ValueInput::Plain(f64::NAN)))?;
    steps.push(record_step(gui, target, key, "invalid"));

    // teardown — adapter replaces the spec; the walked key unmounts.
    unmount(gui)?;
    gui.settle()?;
    steps.push(record_step(gui, target, key, "teardown"));

    Ok(steps)
}

/// One dispatch + settle + intent drain + observe, folded into a
/// [`StoryStep`]. Intents are drained *before* observe so a rejected
/// input's silence is measured against exactly this state's window.
fn record_step<G: SemanticGuiTest>(
    gui: &mut G,
    target: &ScenarioTarget,
    key: &ComponentKey,
    state: &'static str,
) -> StoryStep {
    let intents = gui
        .drain_intents()
        .iter()
        .map(|m| intent_kind(&m.intent))
        .collect();
    match gui.observe(target, key) {
        Ok(snap) => StoryStep {
            state,
            intents,
            committed: snap.committed.map(|v| v.plain),
            display: snap.display.map(|v| v.plain),
            phase: Some(snap.phase),
            mounted: snap.mounted,
            observe_error: None,
        },
        Err(e) => StoryStep {
            state,
            intents,
            committed: None,
            display: None,
            phase: None,
            mounted: false,
            observe_error: Some(error_kind(&e)),
        },
    }
}

fn dispatch_settled<G: SemanticGuiTest>(
    gui: &mut G,
    target: &ScenarioTarget,
    key: &ComponentKey,
    input: ControlInput,
) -> Result<(), ScenarioError> {
    gui.dispatch_input(target, key, input)?;
    gui.settle()
}

/// Stable kind name for a minted [`CommandIntent`] — the trace
/// records kinds, never command ids or payloads.
fn intent_kind(intent: &CommandIntent) -> &'static str {
    match intent {
        CommandIntent::Begin { .. } => "begin",
        CommandIntent::Preview { .. } => "preview",
        CommandIntent::Commit { .. } => "commit",
        CommandIntent::Cancel { .. } => "cancel",
        CommandIntent::Action { .. } => "action",
        _ => "other",
    }
}

/// Stable kind name for a [`ScenarioError`] — recorded on the
/// teardown step where `observe` is expected to fail.
fn error_kind(e: &ScenarioError) -> &'static str {
    match e {
        ScenarioError::NoSuchWindow { .. } => "no_such_window",
        ScenarioError::NoSuchView { .. } => "no_such_view",
        ScenarioError::StaleEpoch { .. } => "stale_epoch",
        ScenarioError::StaleGeneration { .. } => "stale_generation",
        ScenarioError::UnknownComponent { .. } => "unknown_component",
        ScenarioError::View(_) => "view",
        ScenarioError::Driver(_) => "driver",
        ScenarioError::Unsupported { .. } => "unsupported",
    }
}
