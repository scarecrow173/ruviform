//! Integration coverage for the plugin-dock scenario harness
//! (Issue #631 first slice).
//!
//! Two honesty properties are asserted:
//!
//! - **fail-closed**: an unresolvable artifact turns every
//!   artifact-dependent case into `NotRun` — never a silent skip or
//!   fabricated `Pass`;
//! - **honest verdicts**: with the real `test_gain` cdylib present
//!   (the dev-dep edge builds it into `deps/`), load/discover/hash,
//!   the host lifecycle and the session-lifecycle cases pass while
//!   editor attach stays `Unsupported` (the fixture ships no editor)
//!   and dock pixels stay `NotRun`.

use std::path::PathBuf;

use ruviform_plugin_dock_test::{LaneEnvironment, run_all};
use ruviform_ui_test::ScenarioVerdict;

fn verdict_of<'a>(
    reports: &'a [ruviform_ui_test::ScenarioReport],
    case: &str,
) -> &'a ruviform_ui_test::ScenarioReport {
    reports
        .iter()
        .find(|report| report.case.as_ref().is_some_and(|id| id.as_str() == case))
        .unwrap_or_else(|| panic!("missing case report `{case}`"))
}

/// A bogus explicit path must fail closed: resolution fails, so the
/// artifact case and every dependent record `NotRun` — the lane
/// output still carries run/case/driver/verdict evidence.
#[test]
fn missing_artifact_fails_closed_as_not_run() {
    let environment = LaneEnvironment {
        repo_sha: None,
        plugin_path: Some(PathBuf::from("definitely/not/a/real/plugin.dll")),
    };
    let reports = run_all(&environment);
    assert_eq!(reports.len(), 8, "eight plugin-dock cases");

    assert_eq!(
        verdict_of(&reports, "view-mount").verdict,
        ScenarioVerdict::Pass,
        "semantic mount does not depend on the artifact"
    );
    for case in [
        "artifact",
        "host-lifecycle",
        "editor-attach",
        "session-lifecycle",
        "host-terminate",
        "open-or-degrade",
        "dock-pixels",
    ] {
        let report = verdict_of(&reports, case);
        assert_eq!(
            report.verdict,
            ScenarioVerdict::NotRun,
            "case `{case}` must fail closed as NotRun without the artifact"
        );
    }
    // The artifact case detail must name the resolution failure.
    let detail = verdict_of(&reports, "artifact")
        .detail
        .as_deref()
        .unwrap_or_default();
    assert!(detail.contains("not found"), "detail: {detail}");
}

/// With the real cdylib resolvable, the honest-verdict contract
/// holds: load/discover/hash + host lifecycle pass; editor attach is
/// `Unsupported` (fixture has no editor); dock pixels are `NotRun`.
/// When the artifact cannot be resolved in this environment the same
/// assertions degrade to the fail-closed shape — never a fake pass.
#[test]
fn fixture_run_reports_honest_verdicts() {
    let reports = run_all(&LaneEnvironment::default());
    assert_eq!(reports.len(), 8);

    let artifact = verdict_of(&reports, "artifact");
    if artifact.verdict == ScenarioVerdict::NotRun {
        // cdylib absent in this environment — dependents must be
        // NotRun too (covered by the fail-closed test as well).
        for case in [
            "host-lifecycle",
            "editor-attach",
            "session-lifecycle",
            "host-terminate",
            "open-or-degrade",
        ] {
            assert_eq!(verdict_of(&reports, case).verdict, ScenarioVerdict::NotRun);
        }
        return;
    }

    assert_eq!(
        artifact.verdict,
        ScenarioVerdict::Pass,
        "artifact resolve/hash/load/discover: {}",
        artifact.detail.as_deref().unwrap_or("no detail")
    );
    // Artifact evidence: digest + version must be recorded.
    let environment = artifact.environment.as_ref().expect("environment recorded");
    let plugin = &environment["plugin_artifact"];
    assert!(
        plugin["sha256"].as_str().is_some_and(|s| s.len() == 64),
        "sha256 digest recorded: {plugin}"
    );
    assert_eq!(plugin["name"], "test_gain");
    assert!(plugin["version"].as_str().is_some());
    assert_eq!(
        environment["environment_kind"], "real-abi-test-plugin",
        "real ABI execution claims the real kind"
    );

    assert_eq!(
        verdict_of(&reports, "host-lifecycle").verdict,
        ScenarioVerdict::Pass,
        "scan/instantiate/initialize/activate ran against the real binary"
    );

    let attach = verdict_of(&reports, "editor-attach");
    assert!(
        matches!(
            attach.verdict,
            ScenarioVerdict::Unsupported | ScenarioVerdict::NotRun
        ),
        "editor attach must never pass without a real editor: {:?}",
        attach.verdict
    );
    if attach.verdict == ScenarioVerdict::Unsupported {
        let detail = attach.detail.as_deref().unwrap_or_default();
        assert!(
            detail.contains("createView") || detail.contains("Unsupported"),
            "detail must name the honest refusal: {detail}"
        );
    }

    // The session-lifecycle cases ran the real binary end-to-end —
    // `Pass` is honest here because every asserted step executed.
    for case in ["session-lifecycle", "host-terminate", "open-or-degrade"] {
        let report = verdict_of(&reports, case);
        assert_eq!(
            report.verdict,
            ScenarioVerdict::Pass,
            "case `{case}` must pass on the real binary: {:?}",
            report.detail
        );
        let kind = report
            .environment
            .as_ref()
            .and_then(|env| env["environment_kind"].as_str());
        assert_eq!(
            kind,
            Some("real-abi-test-plugin"),
            "case `{case}` claims the real ABI kind only because it ran"
        );
    }

    let pixels = verdict_of(&reports, "dock-pixels");
    assert_eq!(
        pixels.verdict,
        ScenarioVerdict::NotRun,
        "dock pixels stay NotRun — no surface, no capture driver"
    );

    // The fake-slot attach path is contract evidence only — the
    // environment must not claim a real window kind anywhere.
    for report in &reports {
        if let Some(kind) = report
            .environment
            .as_ref()
            .and_then(|env| env["environment_kind"].as_str())
        {
            assert_ne!(kind, "real-window", "no real window ran: {kind}");
        }
    }
}
