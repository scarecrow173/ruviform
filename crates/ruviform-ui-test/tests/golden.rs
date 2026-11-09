//! Issue #314 slice 1 — Golden Utility コアの統合 test。
//!
//! 合成 RGBA8/PNG のみを使い、capture・実 OS・GPU・フォント環境に
//! 依存せず baseline store の candidate→promote→compare→evidence
//! を end-to-end で検証する。semantic oracle・DrawPlan・実
//! screenshot 取得は対象外（capture は slice 3）。

use std::fs;
use std::path::PathBuf;

use ruviform_ui_test::golden::{
    self, BaselineApproval, BaselineStore, CapturePath, ChannelTolerance, DiffPolicy,
    EnvironmentMeta, GoldenKey, GoldenVerdict, MaskRect, RgbaImage,
};
use ruviform_ui_test::proto::Screenshot;

fn key() -> GoldenKey {
    GoldenKey {
        story_id: "LAB-SV-FADER-DOCK".to_string(),
        theme: "synvelis-dark".to_string(),
        state: "idle".to_string(),
        driver_class: "test-headless".to_string(),
        os_class: "test-os".to_string(),
        gpu_class: None,
        scale_factor: 1.0,
        font_class: Some("test-font".to_string()),
        revision: "rev-test".to_string(),
    }
}

fn env() -> EnvironmentMeta {
    EnvironmentMeta {
        os: "test-os".to_string(),
        arch: "test-arch".to_string(),
        os_version: None,
        backend: "test-headless".to_string(),
        backend_detail: None,
        gpu: None,
        scale_factor: 1.0,
        font_set: Some("test-font-set".to_string()),
        capture: CapturePath::Unknown,
        ruviform_revision: "0.0.0-test".to_string(),
        tool_revision: None,
    }
}

fn approval() -> BaselineApproval {
    BaselineApproval {
        approved_by: "integration-test".to_string(),
        note: Some("synthetic baseline".to_string()),
    }
}

fn strict_policy() -> DiffPolicy {
    DiffPolicy {
        channel_tolerance: ChannelTolerance::uniform(0),
        mask: Vec::new(),
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ruviform-golden-it-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn golden_key_drives_deterministic_layout() {
    let k = key();
    // canonical は全要素を含み、file_stem は fs 安全
    let canonical = k.canonical();
    assert!(canonical.contains("LAB-SV-FADER-DOCK"));
    assert!(canonical.contains("synvelis-dark"));
    assert!(canonical.contains("rev-test"));
    let stem = k.file_stem();
    assert!(
        stem.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    );
    // serde round-trip
    let json = serde_json::to_string(&k).unwrap();
    let back: GoldenKey = serde_json::from_str(&json).unwrap();
    assert_eq!(k, back);
}

#[test]
fn screenshot_png_and_rgba_raw_decode() {
    // 合成 PNG を wire 型に載せて decode する（capture ではなく
    // format 変換の検証）。
    let img = RgbaImage::solid(3, 2, [12, 34, 56, 255]);
    let png = img.to_png().unwrap();
    let shot = Screenshot {
        format: "png".to_string(),
        data_base64: base64_encode(&png),
        width: 3,
        height: 2,
    };
    assert_eq!(RgbaImage::from_screenshot(&shot).unwrap(), img);

    let raw = Screenshot {
        format: "rgba-raw".to_string(),
        data_base64: base64_encode(img.pixels()),
        width: 3,
        height: 2,
    };
    assert_eq!(RgbaImage::from_screenshot(&raw).unwrap(), img);

    // 未対応 format は成功扱いしない
    let bad = Screenshot {
        format: "jpeg".to_string(),
        data_base64: base64_encode(&png),
        width: 3,
        height: 2,
    };
    assert!(RgbaImage::from_screenshot(&bad).is_err());
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[test]
fn baseline_lifecycle_candidate_promote_compare_evidence() {
    let base = temp_dir("lifecycle-base");
    let staging = temp_dir("lifecycle-staging");
    let evidence_dir = temp_dir("lifecycle-ev");
    let store = BaselineStore::new(&base);
    let k = key();

    // baseline 不在 → blocked（pass にしない）
    let actual = RgbaImage::solid(8, 8, [200, 200, 200, 255]);
    let cmp = store.compare(&k, &actual, Some(&env())).unwrap();
    assert_eq!(cmp.report.verdict, GoldenVerdict::Blocked);
    assert_eq!(cmp.report.reason, "baseline_missing");

    // candidate → staging のみ。baseline は依然不在
    store
        .write_candidate(&staging, &k, &actual, Some(&env()), &strict_policy())
        .unwrap();
    assert!(store.read(&k).unwrap().is_none());

    // promote（承認必須）→ baseline 出現
    store.promote(&staging, &k, &approval()).unwrap();
    let baseline = store.read(&k).unwrap().expect("promoted baseline");
    assert_eq!(
        baseline.sidecar.approval.as_ref().unwrap().approved_by,
        "integration-test"
    );
    assert_eq!(
        baseline.sidecar.policy.channel_tolerance,
        ChannelTolerance::uniform(0)
    );

    // 同一画像 → pass
    let cmp = store.compare(&k, &actual, Some(&env())).unwrap();
    assert_eq!(cmp.report.verdict, GoldenVerdict::Pass);
    assert_eq!(cmp.report.reason, "match");
    assert!(cmp.report.baseline.is_some());
    let stats = cmp.report.stats.as_ref().unwrap();
    assert_eq!(stats.compared_pixels, 64);
    assert_eq!(stats.mismatched_pixels, 0);

    // 1 画素差分 → fail + reason + diff 画像
    let mut changed = actual.clone();
    changed.set_pixel(3, 3, [201, 200, 200, 255]);
    let cmp = store.compare(&k, &changed, Some(&env())).unwrap();
    assert_eq!(cmp.report.verdict, GoldenVerdict::Fail);
    assert_eq!(cmp.report.reason, "tolerance_exceeded");
    let stats = cmp.report.stats.as_ref().unwrap();
    assert_eq!(stats.mismatched_pixels, 1);
    assert_eq!(
        stats.diff_bbox,
        Some(MaskRect {
            x: 3,
            y: 3,
            width: 1,
            height: 1,
        })
    );
    let diff_img = cmp.diff_image.expect("diff image on same-size compare");
    assert_eq!(diff_img.pixel(3, 3), Some([255, 0, 0, 255]));

    // evidence: env 未設定では副作用なしを確認した上で、明示 path に append
    let ev_path = evidence_dir.join("golden.jsonl");
    let report = &cmp.report;
    let evidence = golden::GoldenEvidence::from(report);
    golden::append_evidence(&ev_path, &evidence).unwrap();
    let line = fs::read_to_string(&ev_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(line.lines().next().unwrap()).unwrap();
    assert_eq!(v["schema"], golden::EVIDENCE_SCHEMA);
    assert_eq!(v["verdict"], "fail");
    assert_eq!(v["reason"], "tolerance_exceeded");
    assert!(v["subject"].as_str().unwrap().contains("LAB-SV-FADER-DOCK"));
    assert!(
        v["environment"]["backend"]
            .as_str()
            .unwrap()
            .contains("test-headless")
    );
    assert!(
        std::path::Path::new(v["baseline"].as_str().unwrap())
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    );

    // emit_evidence は env 未設定なら None（副作用なし）— この test
    // process では RUVIFORM_GOLDEN_EVIDENCE を設定していない。
    assert!(golden::emit_evidence(report).is_none());
    assert!(!evidence_dir.join("golden-evidence.jsonl").exists());

    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_dir_all(&evidence_dir);
}

#[test]
fn unsupported_and_not_run_never_become_pass() {
    let k = key();
    // capture 層が pixel を得られなかった場合の語彙 —
    // headless backend の screenshot は unavailable であり、
    // golden は unsupported/not_run を記録する（pass にしない）。
    let r = golden::unsupported(&k, Some(&env()), "screenshot_unsupported");
    assert_eq!(r.verdict, GoldenVerdict::Unsupported);
    assert_ne!(r.verdict, GoldenVerdict::Pass);
    let r = golden::not_run(&k, Some(&env()), "semantic_oracle_failed");
    assert_eq!(r.verdict, GoldenVerdict::NotRun);
    assert_ne!(r.verdict, GoldenVerdict::Pass);
    // evidence にも verdict がそのまま残る
    let ev = golden::GoldenEvidence::from(&r);
    assert_eq!(ev.verdict, "not_run");
    assert!(ev.stats.is_none());
}

#[test]
fn mask_and_alpha_normalization_in_integration() {
    let base = temp_dir("mask-base");
    let staging = temp_dir("mask-staging");
    let store = BaselineStore::new(&base);
    let mut k2 = key();
    k2.state = "masked".to_string();

    // mask 付き policy で promote
    let mut p = strict_policy();
    p.mask.push(MaskRect {
        x: 0,
        y: 0,
        width: 8,
        height: 8,
    });
    let img = RgbaImage::solid(8, 8, [1, 2, 3, 255]);
    store
        .write_candidate(&staging, &k2, &img, None, &p)
        .unwrap();
    store.promote(&staging, &k2, &approval()).unwrap();

    // 全面 mask → compared=0 → blocked（vacuous pass しない）
    let cmp = store.compare(&k2, &img, None).unwrap();
    assert_eq!(cmp.report.verdict, GoldenVerdict::Blocked);
    assert_eq!(cmp.report.reason, "empty_comparison");

    // alpha=0 正規化: 双方 alpha=0 の RGB 差は mismatch にしない
    let a = RgbaImage::solid(2, 2, [0, 0, 0, 0]);
    let mut b = RgbaImage::solid(2, 2, [255, 255, 255, 0]);
    b.set_pixel(0, 0, [1, 2, 3, 0]);
    let outcome = golden::diff(&a, &b, &strict_policy());
    let golden::DiffOutcome::Compared(r) = outcome else {
        panic!("expected Compared");
    };
    assert_eq!(r.stats.mismatched_pixels, 0);
    assert!(r.within_policy);

    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&staging);
}

// ---------------------------------------------------------------------
// Issue #314 slice 5 — env 正規化・fixture registry 結合・evidence lane
// ---------------------------------------------------------------------

#[test]
fn key_env_normalizes_environment_into_key_classes() {
    use ruviform_ui_test::golden::{GpuMeta, key_env};
    use ruviform_ui_test::lab_story;

    let mut e = env();
    e.os = "  Windows  ".to_string();
    e.gpu = Some(GpuMeta {
        name: Some("Raw Adapter".to_string()),
        vendor: Some("nvidia".to_string()),
        class: Some("dx11".to_string()),
    });
    e.font_set = Some("  Inter-V13  ".to_string());
    let classes = key_env(&e, "gpui-real-window-os-capture").expect("valid env");
    assert_eq!(classes.os_class, "Windows");
    assert_eq!(classes.gpu_class.as_deref(), Some("dx11"));
    assert_eq!(classes.scale_factor, 1.0);
    assert_eq!(classes.font_class.as_deref(), Some("Inter-V13"));

    // fixture 結合: story_id/theme は registry fixture から来る。
    let fixture = lab_story::sv_fader_dock_fixture();
    let k = golden::key_for_fixture(&fixture, "idle", &classes, "rev-1");
    assert_eq!(k.story_id, "LAB-SV-FADER-DOCK");
    assert_eq!(k.theme, "synvelis-dark");
    assert_eq!(k.driver_class, "gpui-real-window-os-capture");
    assert_eq!(k.revision, "rev-1");
}

#[test]
fn registry_mismatches_reports_manifest_drift() {
    use ruviform_ui_test::driver_lab;
    use ruviform_ui_test::golden::{GoldenKeyEnv, key_for_fixture, registry_mismatches};
    use ruviform_ui_test::lab_story;

    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/driver-lab/stories.json");
    let manifest = driver_lab::load_stories(&manifest_path).expect("stories.json loads");

    let classes = GoldenKeyEnv {
        driver_class: "test".to_string(),
        os_class: "test-os".to_string(),
        gpu_class: None,
        scale_factor: 1.0,
        font_class: None,
    };
    let fixture = lab_story::sv_fader_dock_fixture();
    let k = key_for_fixture(&fixture, "idle", &classes, "rev");
    assert!(
        registry_mismatches(&k, &manifest).is_empty(),
        "fixture-derived key must match stories.json"
    );

    let mut drifted = k.clone();
    drifted.state = "no-such-state".to_string();
    let mismatches = registry_mismatches(&drifted, &manifest);
    assert!(mismatches.iter().any(|m| m.field == "state"));
}

#[test]
fn evidence_lane_is_pixel_golden_regardless_of_verdict() {
    let k = key();
    for report in [
        golden::unsupported(&k, Some(&env()), "no_backend"),
        golden::not_run(&k, Some(&env()), "prereq_unmet"),
        golden::blocked(&k, Some(&env()), "baseline_missing", "no baseline"),
    ] {
        let ev = golden::GoldenEvidence::from(&report);
        assert_eq!(ev.lane, golden::EVIDENCE_LANE);
        assert_eq!(ev.lane, "pixel-golden");
        let line = serde_json::to_string(&ev).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["lane"], "pixel-golden");
        assert_eq!(v["schema"], golden::EVIDENCE_SCHEMA);
    }
}
