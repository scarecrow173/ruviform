//! Issue #314 slice 4 — `ScenarioDriver::RealOs` adapter
//! (`RealOsGuiTest`, `real-scenario` feature).
//!
//! Each canonical Driver Lab story mounts into a *real OS window*
//! (`RealOsGuiTest::run` → `GpuiDriver::create_view_window` on the
//! platform loop) and walks the seven shared semantic states through
//! the `SemanticGuiTest` port — the same
//! `expected_lab_story_trace()` oracle the Fake and GPUI-simulated
//! layers assert. Semantic dispatch crosses the UI-thread job seam
//! into `App` context; it is still *semantic* evidence, never
//! pointer/OS-input evidence.
//!
//! The visual layer is the point of this slice: `capture_frame`
//! reads the real window's client-area pixels through
//! `GpuiDriver::capture_window` (`PrintWindow` primary, screen
//! `BitBlt` fallback — Windows only in this slice) and feeds them to
//! `golden_check`/`BaselineStore::compare`. On a fresh store the
//! verdict is `blocked`/`baseline_missing` — proof the pixels
//! reached the compare path — and a second capture after
//! candidate → promote must `pass`, the same loop
//! `golden_capture.rs` verifies at the driver level.
//!
//! Windows + interactive desktop required: on a headless host the
//! window cannot open and `run` reports `UiError::Host` — recorded
//! honestly, never substituted. This target is gated on
//! `real-scenario`, which `gpui-test` deliberately does not enable
//! (the test platform stubs OS capture), so it runs as an auxiliary
//! lane: `cargo test -p ruviform-ui-gpui --features real-scenario
//! --test real_window`.

#![cfg(all(windows, feature = "real-scenario"))]

use std::path::PathBuf;

use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpochAllocator, ViewHost};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{CaptureMeta, RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::driver_lab;
use ruviform_ui_test::golden::{
    BaselineApproval, BaselineStore, CapturePath, ChannelTolerance, DiffPolicy, EnvironmentMeta,
    GoldenKey, GoldenVerdict,
};
use ruviform_ui_test::lab_story::{self, StoryFixture};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    RunId, ScenarioError, ScenarioId, ScenarioReport, ScenarioTarget, SemanticGuiTest, TargetQuery,
    record_golden_step,
};
use ruviform_ui_theme::{ResolvedTheme, ScaleContext, ThemeResolver, TokenDocument};

/// Loads a theme fixture from the shared `ruviform-ui-test` theme
/// JSON — identical to the Fake/GPUI-simulated matrix.
fn resolve_theme(name: &str) -> ResolvedTheme {
    let path = format!(
        "{}/../ruviform-ui-test/fixtures/themes/{name}.dtcg.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let doc = TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    ThemeResolver::resolve(&[doc], "dark").unwrap_or_else(|e| panic!("resolve {name}: {e}"))
}

/// Runs `body` inside one real-window session for `fixture` —
/// mounts the canonical spec as a real OS window, waits the startup
/// settle, then hands the adapter to `body` on the caller thread.
fn run_story_session(
    fixture: &StoryFixture,
    body: impl FnOnce(&mut RealOsGuiTest, WindowId, ScenarioTarget) -> Result<(), ScenarioError>,
) {
    let themed = resolve_theme(fixture.theme).with_scale(ScaleContext {
        scale_factor: 1.0,
        rem_base_px: 16.0,
    });
    let view = ViewInstanceId::new(426).expect("nonzero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(fixture.spec.clone()).expect("mount story spec");
    host.drain_events();

    RealOsGuiTest::run(
        vec![RealWindowMount {
            window: WindowSpec::default(),
            view: ViewMount {
                host,
                theme: Some(themed),
            },
        }],
        RealWindowOptions::default(),
        |gui| {
            let window = gui
                .windows()
                .into_iter()
                .next()
                .expect("one mounted window");
            let target = gui
                .resolve_target(&TargetQuery::new(window, view))
                .expect("resolve story target");
            body(gui, window, target)
        },
    )
    .expect("real-window session")
    .expect("scenario body");
}

/// The story `GoldenKey` — real story/theme ids, the declared state,
/// and the *measured* scale factor. `driver_class` is `real-os`:
/// only pixels from `capture_frame` can produce a `pass`/`fail`
/// verdict on this key.
fn story_golden_key(fixture: &StoryFixture, state: &str, meta: CaptureMeta) -> GoldenKey {
    GoldenKey {
        story_id: fixture.story_id.to_string(),
        theme: fixture.theme.to_string(),
        state: state.to_string(),
        driver_class: "real-os".to_string(),
        os_class: std::env::consts::OS.to_string(),
        gpu_class: None,
        scale_factor: meta.scale_factor,
        font_class: None,
        revision: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// `EnvironmentMeta` from the *measured* capture — backend tag and
/// scale factor come from the frame, never assumed.
fn story_environment(meta: CaptureMeta) -> EnvironmentMeta {
    EnvironmentMeta {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os_version: None,
        backend: "gpui".to_string(),
        backend_detail: Some(format!(
            "OsCapturedFrame.backend = {} (PrintWindow preferred, screen BitBlt fallback)",
            meta.backend
        )),
        gpu: None,
        scale_factor: meta.scale_factor,
        font_set: None,
        capture: CapturePath::RealWindow,
        ruviform_revision: env!("CARGO_PKG_VERSION").to_string(),
        tool_revision: None,
    }
}

/// Fresh per-run store dir — baselines never persist between runs,
/// so `baseline_missing` → candidate → promote → `pass` is exercised
/// every time.
fn golden_store(tag: &str) -> BaselineStore {
    let dir = std::env::temp_dir().join(format!(
        "ruviform-real-window-golden-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    BaselineStore::new(dir)
}

/// Optional durable pixel evidence: `RUVIFORM_REAL_WINDOW_PNG_DIR`
/// names a directory that receives `<story>-idle.png`,
/// `<story>-teardown.png` and a `<story>-meta.jsonl` line. Unset →
/// no artifacts (same env-gate convention as the driver-lab/golden
/// evidence channels).
fn dump_png_evidence(
    story_id: &str,
    idle: &ruviform_ui_test::golden::RgbaImage,
    teardown: &ruviform_ui_test::golden::RgbaImage,
    meta: Option<CaptureMeta>,
) {
    let Ok(dir) = std::env::var("RUVIFORM_REAL_WINDOW_PNG_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("png evidence dir");
    for (name, img) in [("idle", idle), ("teardown", teardown)] {
        let png = img.to_png().expect("encode evidence png");
        std::fs::write(dir.join(format!("{story_id}-{name}.png")), png)
            .expect("write evidence png");
    }
    if let Some(meta) = meta {
        let line = format!(
            "{{\"story\":\"{story_id}\",\"state\":\"teardown\",\"width\":{},\"height\":{},\"scale_factor\":{},\"backend\":\"{}\"}}\n",
            meta.width, meta.height, meta.scale_factor, meta.backend
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{story_id}-meta.jsonl")))
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
            .expect("write evidence meta");
    }
}

fn real_window_story(fixture: &StoryFixture) {
    let staging_root = std::env::temp_dir().join(format!(
        "ruviform-real-window-staging-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&staging_root).expect("staging dir");

    run_story_session(fixture, |gui, window, target| {
        // Pixel layer, idle state: the populated story UI must
        // produce real, non-uniform OS pixels before any gesture.
        let idle_image = match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => image,
            other => panic!(
                "{} idle capture must produce real pixels on a desktop session, got {other:?}",
                fixture.story_id
            ),
        };
        assert!(idle_image.width() > 0 && idle_image.height() > 0);
        let mut distinct = std::collections::BTreeSet::new();
        for y in (0..idle_image.height()).step_by(16) {
            for x in (0..idle_image.width()).step_by(16) {
                if let Some(px) = idle_image.pixel(x, y) {
                    distinct.insert([px[0], px[1], px[2]]);
                }
            }
        }
        assert!(
            distinct.len() > 1,
            "idle frame looks uniform — not real window pixels"
        );

        // Semantic layer: the shared seven-state oracle — identical
        // evidence shape to the Fake/GPUI-simulated layers.
        let walk_key = fixture.walk_key.clone();
        let teardown = fixture.teardown_spec.clone();
        let trace = lab_story::lab_story_walk(gui, &target, &walk_key, move |g| {
            g.update_view(window, teardown)
        })?;
        assert_eq!(
            trace,
            lab_story::expected_lab_story_trace(),
            "{} real-window story trace diverged",
            fixture.story_id,
        );

        // Pixel layer, teardown state: the target pins survive the
        // spec swap (generation stays stable), so capture measures
        // the post-teardown window — real OS pixels, never a draw
        // plan. (The fader story's teardown spec is an empty panel —
        // uniform pixels are legitimate here; the non-uniform check
        // lives on the populated idle frame above.)
        let image = match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => image,
            other => panic!(
                "{} teardown capture must produce real pixels on a desktop session, got {other:?}",
                fixture.story_id
            ),
        };
        assert!(image.width() > 0 && image.height() > 0);
        dump_png_evidence(
            fixture.story_id,
            &idle_image,
            &image,
            gui.last_capture_meta(),
        );

        let meta = gui.last_capture_meta().expect("capture meta");
        let key = story_golden_key(fixture, "teardown", meta);
        let env = story_environment(meta);
        let store = golden_store(fixture.story_id);
        let policy = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(2),
            mask: Vec::new(),
        };
        let approval = BaselineApproval {
            approved_by: "real-window-test".to_string(),
            note: Some("self-approved e2e baseline for slice 4 verification".to_string()),
        };
        let mut report = ScenarioReport::new(
            RunId::new(format!("run-{}", fixture.story_id)),
            ScenarioId::new("lab-story-real-window"),
            None,
            gui.driver(),
        );

        // Golden layer step 1: story-keyed `golden_check` on a fresh
        // store → `blocked`/`baseline_missing` — proof the pixels
        // reached `BaselineStore::compare` (an `Unsupported`/
        // `NotRun` outcome would name a different verdict).
        let comparison = gui.golden_check(&target, &key, Some(&env), &store)?;
        assert_eq!(comparison.report.verdict, GoldenVerdict::Blocked);
        assert_eq!(comparison.report.reason, "baseline_missing");
        record_golden_step(&mut report, &comparison);

        // Candidate → promote with the *same* measured pixels, then
        // a second `golden_check` captures fresh pixels and must
        // `pass` — the promote→compare loop proves repeatability,
        // not a one-off framebuffer read.
        store
            .write_candidate(&staging_root, &key, &image, Some(&env), &policy)
            .map_err(|e| ScenarioError::Driver(format!("write_candidate: {e}")))?;
        store
            .promote(&staging_root, &key, &approval)
            .map_err(|e| ScenarioError::Driver(format!("promote: {e}")))?;
        gui.settle()?;
        let comparison = gui.golden_check(&target, &key, Some(&env), &store)?;
        assert_eq!(
            comparison.report.verdict,
            GoldenVerdict::Pass,
            "second compare must pass against promoted baseline, got {:?}",
            comparison.report.stats
        );
        record_golden_step(&mut report, &comparison);

        // Story-keyed evidence: semantic + pixel layers both `pass`,
        // mechanism names the real-window lane honestly.
        assert_eq!(report.evidence.len(), 2);
        for evidence in &report.evidence {
            assert!(evidence.contains(fixture.story_id), "{evidence}");
            assert!(evidence.contains("real-os"), "{evidence}");
        }
        driver_lab::emit_evidence(
            fixture.story_id,
            "real-os",
            "pass",
            "pass",
            "cargo:test-real-window",
            "RealOsGuiTest: 7-state shared oracle + real OS-window pixel capture → golden_check (baseline_missing → promote → pass)",
        );
        Ok(())
    });
}

#[test]
fn lab_story_sv_fader_dock_real_window() {
    real_window_story(&lab_story::sv_fader_dock_fixture());
}

#[test]
fn lab_story_al_waveform_seek_real_window() {
    real_window_story(&lab_story::al_waveform_seek_fixture());
}
