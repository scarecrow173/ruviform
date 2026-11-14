//! `plugin-dock-scenario` — the Issue #631 plugin-dock lane entry.
//!
//! Runs every `rf631.plugin-dock` case and emits the reports as JSONL:
//! one `run` header line (run id, repo SHA, OS/arch) followed by one
//! `case` line per [`ScenarioReport`] carrying driver, capabilities,
//! required set, verdict, environment (incl. `environment_kind` and
//! plugin-artifact digest/version when the real binary ran) and
//! evidence. Output goes to `--out <path>` or stdout.
//!
//! Exit code: `0` unless any case verdict is `Fail` (a real
//! regression); `NotRun`/`Unsupported` are honest recorded outcomes,
//! not lane failures — a missing artifact must not red the lane, it
//! must be *recorded*.
//!
//! **RT contract:** non-RT only.

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use ruviform_plugin_dock_test::artifact::PLUGIN_PATH_ENV;
use ruviform_plugin_dock_test::{LaneEnvironment, run_all};
use ruviform_ui_test::{ScenarioReport, ScenarioVerdict};
use serde_json::{Map, Value, json};

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                if let Some(path) = args.next() {
                    out = Some(PathBuf::from(path));
                } else {
                    eprintln!("plugin-dock-scenario: --out requires a path");
                    return ExitCode::from(2);
                }
            }
            other => {
                eprintln!("plugin-dock-scenario: unknown argument `{other}`");
                return ExitCode::from(2);
            }
        }
    }

    let environment = LaneEnvironment {
        repo_sha: repo_sha(),
        plugin_path: env::var_os(PLUGIN_PATH_ENV).map(PathBuf::from),
    };
    let reports = run_all(&environment);

    let mut jsonl = String::new();
    jsonl.push_str(&header_line(&environment));
    for report in &reports {
        jsonl.push_str(&case_line(report));
        jsonl.push('\n');
    }

    let write_result = if let Some(path) = &out {
        std::fs::write(path, &jsonl).map_err(|e| format!("{}: {e}", path.display()))
    } else {
        print!("{jsonl}");
        std::io::stdout().flush().map_err(|e| e.to_string())
    };
    if let Err(error) = write_result {
        eprintln!("plugin-dock-scenario: cannot write output: {error}");
        return ExitCode::FAILURE;
    }

    if reports
        .iter()
        .any(|report| report.verdict == ScenarioVerdict::Fail)
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The `run` header: lane identity + host facts + repo SHA.
fn header_line(environment: &LaneEnvironment) -> String {
    let header = json!({
        "type": "run",
        "run_id": ruviform_plugin_dock_test::scenario::RUN_ID,
        "lane": "test-ui-plugin-dock",
        "repo": {
            "sha": environment.repo_sha,
        },
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "plugin_path_env": PLUGIN_PATH_ENV,
    });
    format!(
        "{}\n",
        serde_json::to_string(&header).unwrap_or_else(|_| "{}".to_string())
    )
}

/// One `case` line assembled from the report's public fields —
/// `ScenarioReport` itself is deliberately not `Serialize` (driver
/// types stay adapter-owned), so the JSONL shape lives here at the
/// output boundary.
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
fn repo_sha() -> Option<String> {
    if let Ok(sha) = env::var("GITHUB_SHA") {
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
