//! JSONL rendering shared by the plugin-dock lane binaries —
//! the `run` header plus the per-case [`ScenarioReport`] projection.
//!
//! `ScenarioReport` is deliberately not `Serialize` (driver types
//! stay adapter-owned), so the JSONL shape lives here at the output
//! boundary — one place, every lane binary.
//!
//! **RT contract:** non-RT only.

use std::process::Command;

use ruviform_ui_test::ScenarioReport;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::artifact::PLUGIN_PATH_ENV;

/// Renders the full JSONL document: one `run` header line followed
/// by one `case` line per report, each terminated by `'\n'`.
#[must_use]
pub fn render_jsonl(
    run_id: &str,
    lane: &str,
    repo_sha: Option<&str>,
    reports: &[ScenarioReport],
) -> String {
    let mut out = header_line(run_id, lane, repo_sha);
    for report in reports {
        out.push_str(&case_line(report));
        out.push('\n');
    }
    out
}

/// The `run` header: lane identity + host facts + repo SHA.
fn header_line(run_id: &str, lane: &str, repo_sha: Option<&str>) -> String {
    let header = json!({
        "type": "run",
        "run_id": run_id,
        "lane": lane,
        "repo": {
            "sha": repo_sha,
        },
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "plugin_path_env": PLUGIN_PATH_ENV,
    });
    format!(
        "{}\n",
        serde_json::to_string(&header).unwrap_or_else(|_| "{}".to_string())
    )
}

/// One `case` line assembled from the report's public fields.
fn case_line(report: &ScenarioReport) -> String {
    let object = Map::from_iter([
        ("type".to_string(), json!("case")),
        ("run_id".to_string(), json!(report.run_id)),
        ("scenario".to_string(), json!(report.scenario)),
        (
            "case".to_string(),
            report.case.as_ref().map_or(Value::Null, |case| json!(case)),
        ),
        ("case_seed".to_string(), json!(case_seed(report))),
        ("driver".to_string(), json!(report.driver)),
        (
            "capabilities".to_string(),
            json!(report.capabilities.iter().collect::<Vec<_>>()),
        ),
        (
            "required".to_string(),
            json!(report.required.iter().collect::<Vec<_>>()),
        ),
        ("verdict".to_string(), json!(report.verdict)),
        (
            "expected".to_string(),
            report.expected.clone().unwrap_or(Value::Null),
        ),
        (
            "observed".to_string(),
            report.observed.clone().unwrap_or(Value::Null),
        ),
        ("evidence".to_string(), json!(report.evidence)),
        ("receipts".to_string(), json!(report.receipts)),
        (
            "environment".to_string(),
            report.environment.clone().unwrap_or(Value::Null),
        ),
        (
            "detail".to_string(),
            report.detail.clone().map_or(Value::Null, Value::String),
        ),
        (
            "timeout".to_string(),
            report.timeout.map_or(Value::Null, |timeout| {
                json!({
                    "budget_ms": timeout.budget_ms,
                    "elapsed_ms": timeout.elapsed_ms,
                    "timed_out": timeout.timed_out,
                })
            }),
        ),
        (
            "cleanup".to_string(),
            report.cleanup.clone().unwrap_or(Value::Null),
        ),
    ]);
    // `targets` carries adapter-internal ids — kept out of the JSONL
    // evidence surface on purpose (they are WindowId/ViewInstanceId
    // triples meaningful only inside one driver session).
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_string())
}

/// Deterministic per-case evidence seed — SHA-256 over
/// `run_id|scenario|case`, first 16 hex chars. The lane's cases are
/// deterministic (no RNG), so this is a stable case *identity* for
/// evidence correlation (AC4), not a random seed.
fn case_seed(report: &ScenarioReport) -> String {
    let case = report.case.as_ref().map_or("", |case| case.as_str());
    let canonical = format!(
        "{}|{}|{case}",
        report.run_id.as_str(),
        report.scenario.as_str()
    );
    let digest = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    digest[..16].to_string()
}

/// The repo commit: `GITHUB_SHA` (CI) → `git rev-parse HEAD` → `None`
/// (recorded as honest `null`, never guessed).
#[must_use]
pub fn repo_sha() -> Option<String> {
    if let Ok(sha) = std::env::var("GITHUB_SHA") {
        return Some(sha);
    }
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
}

#[cfg(test)]
mod tests {
    use ruviform_ui::behavior::{CommandReceipt, ReceiptStatus, Revision};
    use ruviform_ui_test::{CaseId, RunId, ScenarioDriver, ScenarioId, ScenarioVerdict};

    use super::*;

    fn report_with_case() -> ScenarioReport {
        let mut report = ScenarioReport::new(
            RunId::new("run-1"),
            ScenarioId::new("scenario-1"),
            Some(CaseId::new("case-1")),
            ScenarioDriver::Fake,
        );
        report.verdict = ScenarioVerdict::Pass;
        report
    }

    #[test]
    fn case_line_carries_receipts_and_case_seed() {
        let mut report = report_with_case();
        report.receipts.push(CommandReceipt {
            command_id: "cmd-1".to_string(),
            status: ReceiptStatus::Applied,
            revision: Some(Revision::Num(7)),
            detail: Some("applied".to_string()),
        });

        let line = case_line(&report);
        let value: Value = serde_json::from_str(&line).expect("case line is JSON");

        assert_eq!(value["case_seed"], json!(case_seed(&report)));
        assert_eq!(value["case_seed"].as_str().map(str::len), Some(16));
        let receipts = value["receipts"].as_array().expect("receipts array");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0]["command_id"], json!("cmd-1"));
        assert_eq!(receipts[0]["status"], json!("applied"));
        assert_eq!(receipts[0]["revision"], json!(7));
    }

    #[test]
    fn case_seed_is_deterministic_and_case_scoped() {
        let a = report_with_case();
        let mut b = report_with_case();
        b.case = Some(CaseId::new("case-2"));

        assert_eq!(case_seed(&a), case_seed(&report_with_case()));
        assert_ne!(case_seed(&a), case_seed(&b));
    }
}
