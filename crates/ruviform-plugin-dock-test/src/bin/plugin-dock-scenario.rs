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
use std::process::ExitCode;

use ruviform_plugin_dock_test::artifact::PLUGIN_PATH_ENV;
use ruviform_plugin_dock_test::jsonl::{render_jsonl, repo_sha};
use ruviform_plugin_dock_test::scenario::RUN_ID;
use ruviform_plugin_dock_test::{LaneEnvironment, run_all};
use ruviform_ui_test::ScenarioVerdict;

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
    let jsonl = render_jsonl(
        RUN_ID,
        "test-ui-plugin-dock",
        environment.repo_sha.as_deref(),
        &reports,
    );

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
