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

use std::collections::BTreeSet;
use std::path::PathBuf;

use ruviform_plugin_dock_test::{HEADLESS_MATRIX, LaneEnvironment, run_all};
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

/// The catalog is the single source `run_all` projects: declared
/// bindings, case ids, order, drivers, and required capabilities
/// must match the produced reports exactly — drift fails here, not
/// in evidence review.
#[test]
fn catalog_matches_run_all_reports() {
    let cells: Vec<_> = HEADLESS_MATRIX.cells().collect();
    assert_eq!(cells.len(), 9, "nine catalog cells");

    // The bogus path keeps the run artifact-free and fast; the
    // catalog↔report correspondence does not depend on verdicts.
    let reports = run_all(&LaneEnvironment {
        repo_sha: None,
        plugin_path: Some(PathBuf::from("definitely/not/a/real/plugin.dll")),
    });
    assert_eq!(reports.len(), cells.len(), "one report per cell");

    for ((entry, binding), report) in cells.iter().zip(reports.iter()) {
        assert_eq!(
            report.case.as_ref().map(ruviform_ui_test::CaseId::as_str),
            Some(entry.case),
            "case id drift"
        );
        assert_eq!(report.scenario.as_str(), entry.scenario);
        assert_eq!(binding.driver, report.driver, "driver drift");
        assert_eq!(
            entry.required.iter().copied().collect::<BTreeSet<_>>(),
            report.required,
            "required-capability drift for `{}`",
            entry.case
        );
    }
}

/// `project()` mirrors `run_scenario`'s pre-flight from declared
/// data only: every headless cell's `required` is covered by its
/// binding's declared capabilities — the lane really executes them,
/// so an honest catalog marks every cell runnable. A case that
/// could not run would surface here as `runnable == false` before
/// any report exists.
#[test]
fn catalog_projection_covers_every_declared_requirement() {
    let projections = HEADLESS_MATRIX.project();
    assert_eq!(projections.len(), 9);
    for projection in &projections {
        assert!(
            projection.runnable,
            "case `{}` is not runnable on `{:?}`: missing {:?}",
            projection.entry.case, projection.driver, projection.missing,
        );
        assert_eq!(projection.missing, None);
    }
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
    assert_eq!(reports.len(), 9, "nine plugin-dock cases");

    assert_eq!(
        verdict_of(&reports, "view-mount").verdict,
        ScenarioVerdict::Pass,
        "semantic mount does not depend on the artifact"
    );
    assert_eq!(
        verdict_of(&reports, "command-receipt").verdict,
        ScenarioVerdict::Pass,
        "semantic receipt flow does not depend on the artifact"
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

/// The Fake-driver command-receipt case is deterministic and
/// artifact-independent: it must pass and populate `receipts`
/// with the two decoded command results (applied + rejected).
fn assert_command_receipt(report: &ruviform_ui_test::ScenarioReport) {
    assert_eq!(
        report.verdict,
        ScenarioVerdict::Pass,
        "command-receipt must pass on the Fake driver: {:?}",
        report.detail
    );
    assert_eq!(
        report.receipts.len(),
        2,
        "applied + rejected receipts recorded: {:?}",
        report.receipts
    );
    assert!(
        report
            .receipts
            .iter()
            .any(|r| r.status == ruviform_ui::behavior::ReceiptStatus::Applied),
        "an applied receipt must be recorded"
    );
    assert!(
        report
            .receipts
            .iter()
            .any(|r| r.status == ruviform_ui::behavior::ReceiptStatus::Rejected),
        "a rejected receipt must be recorded"
    );
}

/// With the real cdylib resolvable, the honest-verdict contract
/// holds: load/discover/hash + host lifecycle pass; editor attach is
/// `Unsupported` (fixture has no editor); dock pixels are `NotRun`.
/// When the artifact cannot be resolved in this environment the same
/// assertions degrade to the fail-closed shape — never a fake pass.
#[test]
fn fixture_run_reports_honest_verdicts() {
    let reports = run_all(&LaneEnvironment::default());
    assert_eq!(reports.len(), 9);

    assert_command_receipt(verdict_of(&reports, "command-receipt"));

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
        environment["environment_kind"], "test-plugin",
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
            Some("test-plugin"),
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
