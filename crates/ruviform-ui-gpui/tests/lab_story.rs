//! Issue #426 — Driver Lab story fixtures on the GPUI-simulated
//! adapter (`GpuiGuiTest` / `GpuiObserver`, `gpui-test` feature).
//!
//! Each story's canonical `ViewSpec` — the same definition the Fake
//! layer walks, via `ruviform_ui_test::lab_story` — mounts into a real
//! GPUI window (`create_view_window` → `RvfView` → `ViewHost`) and
//! walks the seven semantic states through the shared
//! `SemanticGuiTest` port. The trace must equal
//! `expected_lab_story_trace()`, the single oracle both adapters
//! assert, for every {theme, theme-scale} cell of the story matrix.
//!
//! Evidence layers stay separate on purpose:
//!
//! - *semantic*: the intent/snapshot trace above — `dispatch_input`
//!   bypasses hit-testing and is never pointer/OS/pixel evidence.
//! - *draw plan*: `GpuiObserver::drawn()` must be identical across
//!   the whole matrix — a render-plan observation, not pixels.
//! - *pixel*: `gpui-test` has no framebuffer readback, so
//!   `golden_check` honestly reports `Unsupported`; the story-keyed
//!   `GoldenKey` still lands in scenario evidence via
//!   `record_golden_step`. The real-window pixel path for the same
//!   story keys lives in `golden_capture.rs` (Windows, manual lane).
//!
//! Scale honesty: the theme scale axis enters as
//! `ResolvedTheme::with_scale` `ScaleContext` — tokens are invariant
//! under it (asserted per cell, same rule as the Fake matrix), and
//! nothing in the GPUI semantic/draw path consumes the scale hint.
//! The gpui test platform fixes `window.scale_factor` at 2.0
//! (gpui-0.2.2 `src/platform/test/window.rs`), so scale-1.0 *window
//! pixels* are a Visual/real-os lane concern (#313/#314), not this
//! layer's claim.

#![cfg(feature = "gpui-test")]

use ruviform_ui::behavior::ValueInput;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpochAllocator, ViewHost};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{GpuiGuiTest, ViewMount};
use ruviform_ui_test::driver_lab;
use ruviform_ui_test::golden::{BaselineStore, GoldenKey, GoldenVerdict};
use ruviform_ui_test::lab_story::{self, StoryFixture};
use ruviform_ui_test::scenario::visual::VisualGuiTest;
use ruviform_ui_test::{
    RunId, ScenarioError, ScenarioId, ScenarioReport, ScenarioTarget, SemanticGuiTest, TargetQuery,
    parity_gesture_scenario, record_golden_step,
};
use ruviform_ui_theme::{ResolvedTheme, ScaleContext, ThemeResolver, TokenDocument};

/// Story matrix axes — both product themes × both declared scale
/// factors, mirroring `stories.json` `scaleFactor: [1.0, 2.0]` and the
/// Fake-layer matrix.
const THEMES: [&str; 2] = ["synvelis-dark", "audiolens-dark"];
const SCALES: [f64; 2] = [1.0, 2.0];

/// The gpui test platform's fixed window scale factor
/// (gpui-0.2.2 `src/platform/test/window.rs` `scale_factor() -> 2.0`).
/// Used for story `GoldenKey`s on this layer — the *measured* value
/// belongs to the real-window capture lane.
const GPUI_TEST_SCALE_FACTOR: f64 = 2.0;

/// Loads a theme fixture from the shared `ruviform-ui-test` theme
/// JSON — the same documents the Fake-layer matrix resolves.
fn resolve_theme(name: &str) -> ResolvedTheme {
    let path = format!(
        "{}/../ruviform-ui-test/fixtures/themes/{name}.dtcg.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let doc = TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    ThemeResolver::resolve(&[doc], "dark").unwrap_or_else(|e| panic!("resolve {name}: {e}"))
}

/// Mounts `fixture.spec` into a fresh `GpuiGuiTest` window with
/// `theme_name` resolved at `scale_factor` — one adapter per matrix
/// cell so windows/epochs never collide.
fn mount_matrix_cell(
    fixture: &StoryFixture,
    theme_name: &str,
    scale_factor: f64,
) -> (GpuiGuiTest, WindowId, ScenarioTarget) {
    let base = resolve_theme(theme_name);
    let themed = base.with_scale(ScaleContext {
        scale_factor,
        rem_base_px: 16.0,
    });
    // Scale is a mapper-side hint only: tokens must be identical —
    // the same guard the Fake-layer matrix applies.
    assert_eq!(themed.scale().scale_factor, scale_factor);
    assert_eq!(
        themed.tokens().collect::<Vec<_>>(),
        base.tokens().collect::<Vec<_>>(),
        "{theme_name} tokens must not depend on scale context"
    );

    let mut gui = GpuiGuiTest::new();
    let view = ViewInstanceId::new(426).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(fixture.spec.clone()).expect("mount story spec");
    host.drain_events();
    let window = gui
        .observer_mut()
        .driver_mut()
        .create_view_window(
            &WindowSpec::default(),
            ViewMount {
                host,
                theme: Some(themed),
            },
        )
        .expect("create_view_window");
    gui.settle().expect("settle mount");
    let target = gui
        .resolve_target(&TargetQuery::new(window, view))
        .expect("resolve target");
    (gui, window, target)
}

/// The story matrix for one fixture: every {theme, scale} cell mounts
/// the canonical spec, records the draw plan (must be identical
/// across the matrix — draw-plan evidence, kept separate from
/// pixels), and walks the seven semantic states against the shared
/// oracle. Returns the reference draw plan so callers can assert
/// non-vacuity — a story whose components emit no `DrawnPlan` makes
/// the equality check trivially true, and that must be stated, not
/// implied.
fn run_story_matrix(fixture: &StoryFixture) -> Vec<ruviform_ui_gpui::ComponentDrawn> {
    let mut reference_drawn: Option<Vec<ruviform_ui_gpui::ComponentDrawn>> = None;
    for theme_name in THEMES {
        for scale_factor in SCALES {
            let (mut gui, window, target) = mount_matrix_cell(fixture, theme_name, scale_factor);

            // Draw-plan layer: identical plans across the whole matrix.
            let drawn = gui.observer().drawn(window).expect("drawn plans");
            match &reference_drawn {
                Some(want) => assert_eq!(
                    &drawn.components, want,
                    "{} draw plan diverged @ {theme_name} x scale {scale_factor}",
                    fixture.story_id,
                ),
                None => reference_drawn = Some(drawn.components),
            }

            // Semantic layer: the shared oracle trace.
            let walk_key = fixture.walk_key.clone();
            let teardown = fixture.teardown_spec.clone();
            let trace = lab_story::lab_story_walk(&mut gui, &target, &walk_key, move |gui| {
                gui.observer()
                    .driver()
                    .update_view(window, teardown)
                    .map_err(ScenarioError::from)
            })
            .expect("story walk");
            assert_eq!(
                trace,
                lab_story::expected_lab_story_trace(),
                "{} story trace diverged @ {theme_name} x scale {scale_factor}",
                fixture.story_id,
            );
        }
    }
    reference_drawn.expect("matrix ran")
}

#[test]
fn lab_story_sv_fader_dock_gpui_matrix() {
    let plans = run_story_matrix(&lab_story::sv_fader_dock_fixture());
    // Fader/Tabs emit no `DrawnPlan` (only Meter/TimeAxis/Waveform/
    // Spectrogram/Label/Table/Button do), so the draw-plan layer is
    // empty by construction for this story — the semantic oracle is
    // the evidence here, and the empty plan set is asserted rather
    // than implied.
    assert!(
        plans.is_empty(),
        "Fader/Tabs story must emit no draw plans, got {plans:?}"
    );
    driver_lab::emit_evidence(
        "LAB-SV-FADER-DOCK",
        "gpui",
        "pass",
        "pass",
        "mise-task:test-ui-gpui",
        "2 themes x 2 scale contexts x 7-state shared oracle via GpuiGuiTest (draw-plan layer empty by construction: Fader/Tabs emit no DrawnPlan)",
    );
}

#[test]
fn lab_story_al_waveform_seek_gpui_matrix() {
    let plans = run_story_matrix(&lab_story::al_waveform_seek_fixture());
    // The Waveform component emits a `DrawnPlan::Waveform` — the
    // non-empty plan set keeps the draw-plan equality check
    // non-vacuous for this story.
    assert!(!plans.is_empty(), "Waveform story must emit draw plans");
    assert!(
        plans
            .iter()
            .any(|c| matches!(c.plan, ruviform_ui_gpui::DrawnPlan::Waveform { .. })),
        "expected a Waveform draw plan, got {plans:?}"
    );
    driver_lab::emit_evidence(
        "LAB-AL-WAVEFORM-SEEK",
        "gpui",
        "pass",
        "pass",
        "mise-task:test-ui-gpui",
        "2 themes x 2 scale contexts x 7-state shared oracle via GpuiGuiTest + Waveform draw-plan invariance (geometry/bins, not pixels)",
    );
}

// ---------------------------------------------------------------------
// story-keyed golden link (Visual layer stays honest)
// ---------------------------------------------------------------------

fn golden_store() -> (std::path::PathBuf, BaselineStore) {
    let dir = std::env::temp_dir().join(format!(
        "ruviform-lab-story-golden-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    (dir.clone(), BaselineStore::new(dir))
}

/// The story `GoldenKey` — real story/theme ids from the fixture, the
/// declared state, and the platform-fixed test scale. `driver_class`
/// names the simulated layer honestly; it is never a real-OS or
/// pixel-capture claim.
fn story_golden_key(fixture: &StoryFixture, state: &str) -> GoldenKey {
    GoldenKey {
        story_id: fixture.story_id.to_string(),
        theme: fixture.theme.to_string(),
        state: state.to_string(),
        driver_class: "gpui-simulated".to_string(),
        os_class: std::env::consts::OS.to_string(),
        gpu_class: None,
        scale_factor: GPUI_TEST_SCALE_FACTOR,
        font_class: None,
        revision: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Story-keyed `golden_check` on the GPUI-simulated adapter: the
/// `VisualGuiTest` port honestly reports `Unsupported` (gpui-test has
/// no pixel readback) while the story `GoldenKey` still lands in the
/// scenario evidence — the #314 link shape every layer shares. Covers
/// the `idle` and `commit` states so the state axis is exercised too.
fn story_golden_link(fixture: &StoryFixture) {
    let (mut gui, _window, target) =
        mount_matrix_cell(fixture, fixture.theme, GPUI_TEST_SCALE_FACTOR);
    let (_dir, store) = golden_store();
    let mut report = ScenarioReport::new(
        RunId::new(format!("run-{}", fixture.story_id)),
        ScenarioId::new("lab-story-golden"),
        None,
        gui.driver(),
    );

    // idle — before any gesture.
    let comparison = gui
        .golden_check(&target, &story_golden_key(fixture, "idle"), None, &store)
        .expect("golden_check idle");
    assert_eq!(comparison.report.verdict, GoldenVerdict::Unsupported);
    assert_ne!(comparison.report.reason, "");
    record_golden_step(&mut report, &comparison);

    // commit — after the shared parity gesture promotes a value.
    parity_gesture_scenario(
        &mut gui,
        &target,
        &fixture.walk_key,
        "golden-g1",
        ValueInput::Plain(0.8),
    )
    .expect("parity gesture");
    let comparison = gui
        .golden_check(&target, &story_golden_key(fixture, "commit"), None, &store)
        .expect("golden_check commit");
    assert_eq!(comparison.report.verdict, GoldenVerdict::Unsupported);
    record_golden_step(&mut report, &comparison);

    // The story key — not a probe id — is what lands in evidence.
    assert_eq!(report.evidence.len(), 2);
    for (evidence, state) in report.evidence.iter().zip(["idle", "commit"]) {
        assert!(evidence.contains(fixture.story_id), "{evidence}");
        assert!(evidence.contains(fixture.theme), "{evidence}");
        assert!(evidence.contains(&format!("/{state}/")), "{evidence}");
        assert!(evidence.contains("gpui-simulated"), "{evidence}");
    }
}

#[test]
fn lab_story_sv_fader_dock_gpui_golden_link() {
    story_golden_link(&lab_story::sv_fader_dock_fixture());
}

#[test]
fn lab_story_al_waveform_seek_gpui_golden_link() {
    story_golden_link(&lab_story::al_waveform_seek_fixture());
}
