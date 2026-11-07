//! Issue #390 driver boundary guard — 正負 fixture による検証。
//!
//! 各 fixture は `fixtures/driver-lab/boundary/` 配下の独立
//! mini-workspace（path dep のみ・`--offline` で解決可能）。
//! guard が ruleset に従って依存グラフ + public signature を検査し、
//! 期待する finding code を出すことを検証する。

use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use ruviform_ui_test::driver_boundary::{self, GuardReport};

fn fixture_dir(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/driver-lab/boundary")
        .join(rel)
}

fn run_guard(rel: &str) -> GuardReport {
    let dir = fixture_dir(rel);
    let ruleset =
        driver_boundary::load_ruleset(&dir.join("boundary.json")).expect("ruleset が読めること");
    let metadata = MetadataCommand::new()
        .manifest_path(dir.join("Cargo.toml"))
        .other_options(vec!["--offline".to_string()])
        .exec()
        .expect("cargo metadata が解決できること");
    driver_boundary::run(&ruleset, &metadata)
}

fn codes(report: &GuardReport) -> Vec<&'static str> {
    report
        .checks
        .iter()
        .flat_map(|c| c.findings.iter().map(|f| f.code))
        .collect()
}

fn check_findings<'a>(report: &'a GuardReport, check: &str) -> Vec<&'a driver_boundary::Finding> {
    report
        .checks
        .iter()
        .find(|c| c.id == check)
        .map(|c| c.findings.iter().collect())
        .unwrap_or_default()
}

/// positive: allowlisted adapter が optional+feature 経由で
/// forbidden dep を持ち、common は clean — verdict pass。
#[test]
fn positive_allowlisted_adapter_passes() {
    let report = run_guard("positive");
    assert_eq!(report.verdict, "pass", "findings: {:?}", codes(&report));
    for check in &report.checks {
        assert_eq!(
            check.status,
            "pass",
            "check {}: {:?}",
            check.id,
            check.findings.len()
        );
    }
}

/// negative: protected crate の direct forbidden dep 宣言。
#[test]
fn negative_direct_dep_detected() {
    let report = run_guard("negative/direct-dep");
    assert_eq!(report.verdict, "fail");
    let codes = codes(&report);
    assert!(codes.contains(&"forbidden-dep-direct"), "{codes:?}");
    // common -> gpui は非 optional 辺なので transitive でも検出。
    assert!(codes.contains(&"forbidden-dep-transitive"), "{codes:?}");
}

/// negative: 中間 crate 経由の transitive 到達（direct 宣言なし）。
#[test]
fn negative_transitive_dep_detected() {
    let report = run_guard("negative/transitive-dep");
    assert_eq!(report.verdict, "fail");
    let codes = codes(&report);
    assert!(codes.contains(&"forbidden-dep-transitive"), "{codes:?}");
    assert!(!codes.contains(&"forbidden-dep-direct"), "{codes:?}");
    let dep = check_findings(&report, "dep-graph");
    assert!(
        dep.iter()
            .any(|f| f.detail.contains("common -> middle -> gpui")),
        "witness path を含むこと: {dep:?}"
    );
}

/// negative: optional 辺のみ経由の conditional 到達。
#[test]
fn negative_conditional_dep_detected() {
    let report = run_guard("negative/conditional-dep");
    assert_eq!(report.verdict, "fail");
    let codes = codes(&report);
    assert!(codes.contains(&"forbidden-dep-conditional"), "{codes:?}");
    assert!(!codes.contains(&"forbidden-dep-transitive"), "{codes:?}");
}

/// negative: grant 外 feature による forbidden dep 有効化。
#[test]
fn negative_feature_leak_detected() {
    let report = run_guard("negative/feature-leak");
    assert_eq!(report.verdict, "fail");
    let dep = check_findings(&report, "dep-graph");
    assert!(
        dep.iter()
            .any(|f| f.code == "forbidden-dep-feature" && f.detail.contains("sneaky")),
        "{dep:?}"
    );
    // `gpui` feature 自体は grant 済み — `sneaky` のみ違反。
    assert!(
        !dep.iter()
            .any(|f| f.code == "forbidden-dep-feature"
                && f.detail.contains("`gpui` が forbidden dep")),
        "{dep:?}"
    );
}

/// negative: allowlisted dep の非 optional 宣言。
#[test]
fn negative_not_optional_detected() {
    let report = run_guard("negative/not-optional");
    assert_eq!(report.verdict, "fail");
    let codes = codes(&report);
    assert!(codes.contains(&"forbidden-dep-not-optional"), "{codes:?}");
}

/// negative: dev-dependencies の forbidden 宣言も検出。
#[test]
fn negative_dev_dep_detected() {
    let report = run_guard("negative/dev-dep");
    assert_eq!(report.verdict, "fail");
    let dep = check_findings(&report, "dep-graph");
    assert!(
        dep.iter()
            .any(|f| f.code == "forbidden-dep-direct" && f.detail.contains("dev")),
        "dev-dep が kind=dev で検出されること: {dep:?}"
    );
}

/// negative: 公開 signature の forbidden path root 漏出。
/// dep 宣言なし（dep-graph は clean）でも signature 検査が検出する。
#[test]
fn negative_signature_leak_detected() {
    let report = run_guard("negative/signature-leak");
    assert_eq!(report.verdict, "fail");
    let dep = check_findings(&report, "dep-graph");
    assert!(dep.is_empty(), "dep 宣言違反なしのはず: {dep:?}");
    let sig = check_findings(&report, "signature");
    // pub fn 戻り型 / pub field / re-export trait bound / impl method。
    for needle in ["gpui::Entity", "tao::Size", "wry::Into", "impl Pub"] {
        assert!(
            sig.iter().any(|f| f.detail.contains(needle)),
            "`{needle}` を含む finding が必要: {sig:?}"
        );
    }
    // private type への impl は public API でないため検出しない。
    assert!(
        !sig.iter().any(|f| f.detail.contains("impl Priv")),
        "private type impl は対象外: {sig:?}"
    );
}

/// negative: dep.features による adapter toolkit feature 起動 +
/// composition-root なしの adapter 依存。
#[test]
fn negative_adapter_feature_activation_detected() {
    let report = run_guard("negative/adapter-feature");
    assert_eq!(report.verdict, "fail");
    let codes = codes(&report);
    assert!(
        codes.contains(&"forbidden-dep-adapter-feature"),
        "{codes:?}"
    );
    assert!(codes.contains(&"adapter-consumer"), "{codes:?}");
}

/// live workspace 自体が ruleset を pass すること（UI-DVR-01 の
/// driverless build 証拠の片側。`check-ui-driver-boundary` task の
/// 実実行と同一検査）。
#[test]
fn live_workspace_passes_boundary() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let ruleset = driver_boundary::load_ruleset(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/driver-lab/boundary.json"),
    )
    .expect("repo ruleset");
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("workspace cargo metadata");
    let report = driver_boundary::run(&ruleset, &metadata);
    assert_eq!(
        report.verdict, "pass",
        "live workspace が boundary ruleset を pass すること: {:?}",
        report.checks
    );
}
