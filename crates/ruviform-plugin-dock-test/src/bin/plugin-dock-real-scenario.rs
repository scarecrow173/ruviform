//! `plugin-dock-real-scenario` — the Issue #631 real-window
//! plugin-dock lane entry (feature `real-dock`).
//!
//! Runs every `rf631.plugin-dock-real` case inside one
//! `RealOsGuiTest` session — real OS windows on the interactive
//! desktop — and emits the reports as JSONL in the same shape as
//! `plugin-dock-scenario` (shared `jsonl` renderer): one `run`
//! header line followed by one `case` line per [`ScenarioReport`].
//! Output goes to `--out <path>` or stdout.
//!
//! `--pixel-artifacts <dir>` is opt-in: the `real-dcomp-pixel` case
//! then writes each captured frame as a PNG under `<dir>` (created
//! on demand). Without it the case records pixel statistics only —
//! the canonical lane never writes artifacts.
//!
//! Exit code: `0` unless any case verdict is `Fail`; `NotRun`/
//! `Unsupported` are honest recorded outcomes — a headless host
//! records `NotRun` + `observed=environment_blocked` for every case
//! (fail-closed), which must not red the lane. A `Fail` verdict —
//! e.g. `real-dcomp-pixel` under the known unresolved GPUI `DComp`
//! topmost occlusion — exits `1` by design: that is the honest
//! regression signal, not a lane defect.
//!
//! **RT contract:** non-RT only — opens real OS windows.

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use ruviform_plugin_dock_test::LaneEnvironment;
use ruviform_plugin_dock_test::artifact::PLUGIN_PATH_ENV;
use ruviform_plugin_dock_test::jsonl::{render_jsonl, repo_sha};
use ruviform_plugin_dock_test::real_dock::{RUN_ID, run_all_with_pixel_artifacts};
use ruviform_ui_test::ScenarioVerdict;

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut pixel_artifacts: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                if let Some(path) = args.next() {
                    out = Some(PathBuf::from(path));
                } else {
                    eprintln!("plugin-dock-real-scenario: --out requires a path");
                    return ExitCode::from(2);
                }
            }
            "--pixel-artifacts" => {
                if let Some(path) = args.next() {
                    pixel_artifacts = Some(PathBuf::from(path));
                } else {
                    eprintln!("plugin-dock-real-scenario: --pixel-artifacts requires a path");
                    return ExitCode::from(2);
                }
            }
            other => {
                eprintln!("plugin-dock-real-scenario: unknown argument `{other}`");
                return ExitCode::from(2);
            }
        }
    }

    let environment = LaneEnvironment {
        repo_sha: repo_sha(),
        plugin_path: env::var_os(PLUGIN_PATH_ENV).map(PathBuf::from),
    };
    let reports = run_all_with_pixel_artifacts(&environment, pixel_artifacts.as_deref());
    let jsonl = render_jsonl(
        RUN_ID,
        "test-ui-plugin-dock-real",
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
        eprintln!("plugin-dock-real-scenario: cannot write output: {error}");
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
