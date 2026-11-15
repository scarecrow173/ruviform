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
