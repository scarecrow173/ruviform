//! `plugin-dock-console` — the Issue #631 headless scenario console.
//!
//! Consumer of the merged scenario catalog ([`HEADLESS_MATRIX`]):
//!
//! - `--list` prints `ScenarioMatrix::project()` as one JSON catalog
//!   row per line — a capability projection, never execution
//!   evidence (no verdict vocabulary is emitted).
//! - `--list --include-real` appends the real-window dock lane's
//!   twelve catalog rows (`session_required`, `lane`) — requires the
//!   `real-dock` feature; without it the request is refused with
//!   exit 2 rather than silently omitting rows.
//! - `--run [--case <id>] [--scenario <id>] [--driver <name>]`
//!   filters `HEADLESS_MATRIX.cells()` by selector (each flag is
//!   repeatable: OR within a flag, AND across flags) and runs the
//!   selection through `scenario::run_selected` +
//!   `jsonl::render_jsonl` — the same catalog execution path the
//!   lane binary uses, with `lane` recording `plugin-dock-console`
//!   so a filtered run is never mistaken for full-lane evidence.
//!
//! Exit code: `2` on CLI misuse, unknown selectors, selector
//! combinations matching zero headless cells, or real-lane run
//! attempts; `1` on output-write failure or when any report verdict
//! is `Fail`; `0` otherwise (`NotRun`/`Unsupported`/`Blocked` are
//! recorded outcomes, not lane failures).
//!
//! **RT contract:** non-RT only.

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use ruviform_plugin_dock_test::LaneEnvironment;
use ruviform_plugin_dock_test::artifact::PLUGIN_PATH_ENV;
use ruviform_plugin_dock_test::jsonl::{render_jsonl, repo_sha};
use ruviform_plugin_dock_test::real_catalog;
use ruviform_plugin_dock_test::scenario::{HEADLESS_MATRIX, RUN_ID, run_selected};
use ruviform_ui_test::{ScenarioDriver, ScenarioVerdict};
use serde_json::json;

/// Lane label written into the JSONL `run` header — distinguishes
/// console output from the full lane's `test-ui-plugin-dock`.
const CONSOLE_LANE: &str = "plugin-dock-console";

const USAGE: &str = "usage: plugin-dock-console (--list [--include-real] | \
     --run [--case <id>]... [--scenario <id>]... [--driver <name>]...) [--out <path>]";

/// Parsed command line — exactly one of `--list` / `--run`.
struct Options {
    /// `--list` mode.
    list: bool,
    /// `--include-real` (list mode only).
    include_real: bool,
    /// `--run` mode.
    run: bool,
    /// Repeatable `--case` values.
    cases: Vec<String>,
    /// Repeatable `--scenario` values.
    scenarios: Vec<String>,
    /// Repeatable `--driver` values.
    drivers: Vec<String>,
    /// `--out <path>`; `None` writes to stdout.
    out: Option<PathBuf>,
}

fn main() -> ExitCode {
    let Some(options) = parse_args() else {
        return ExitCode::from(2);
    };
    if let Err(message) = validate(&options) {
        eprintln!("plugin-dock-console: {message}");
        return ExitCode::from(2);
    }
    if options.list {
        list(&options)
    } else {
        run(&options)
    }
}

/// Thin `env::args` parse matching the lane binaries — `None`
/// reports the error to stderr and yields exit 2.
fn parse_args() -> Option<Options> {
    let mut options = Options {
        list: false,
        include_real: false,
        run: false,
        cases: Vec::new(),
        scenarios: Vec::new(),
        drivers: Vec::new(),
        out: None,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list" => options.list = true,
            "--include-real" => options.include_real = true,
            "--run" => options.run = true,
            "--case" | "--scenario" | "--driver" => {
                let Some(value) = args.next() else {
                    eprintln!("plugin-dock-console: {arg} requires a value\n{USAGE}");
                    return None;
                };
                match arg.as_str() {
                    "--case" => options.cases.push(value),
                    "--scenario" => options.scenarios.push(value),
                    _ => options.drivers.push(value),
                }
            }
            "--out" => {
                let Some(value) = args.next() else {
                    eprintln!("plugin-dock-console: --out requires a value\n{USAGE}");
                    return None;
                };
                options.out = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => {
                eprintln!("plugin-dock-console: unknown argument `{arg}`\n{USAGE}");
                return None;
            }
        }
    }
    Some(options)
}

/// Command-mode validation — every misuse is exit 2.
fn validate(options: &Options) -> Result<(), String> {
    if options.list == options.run {
        return Err(format!(
            "exactly one of --list / --run is required\n{USAGE}"
        ));
    }
    if options.include_real && !options.list {
        return Err("--include-real is only valid with --list".to_string());
    }
    if !options.run
        && (!options.cases.is_empty()
            || !options.scenarios.is_empty()
            || !options.drivers.is_empty())
    {
        return Err("selectors are only valid with --run".to_string());
    }
    Ok(())
}

/// kebab-case driver name — the same spelling `ScenarioDriver`
/// serializes to in catalog rows and spec extras.
fn driver_name(driver: ScenarioDriver) -> String {
    serde_json::to_value(driver)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// `--list` — capability projection only: one JSON row per catalog
/// cell, no verdict vocabulary, no claim of execution.
fn list(options: &Options) -> ExitCode {
    let mut output = String::new();
    for projection in HEADLESS_MATRIX.project() {
        let row = json!({
            "type": "catalog",
            "case": projection.entry.case,
            "scenario": projection.entry.scenario,
            "driver": projection.driver,
            "required": projection.entry.required,
            "runnable": projection.runnable,
            "missing": projection.missing,
        });
        output.push_str(&row.to_string());
        output.push('\n');
    }
    if options.include_real {
        #[cfg(feature = "real-dock")]
        {
            for row in real_catalog::REAL_CATALOG {
                let line = json!({
                    "type": "catalog",
                    "case": row.case,
                    "scenario": row.scenario,
                    "driver": real_catalog::DRIVER,
                    "required": row.required,
                    "runnable": serde_json::Value::Null,
                    "missing": serde_json::Value::Null,
                    "session_required": true,
                    "lane": real_catalog::LANE,
                });
                output.push_str(&line.to_string());
                output.push('\n');
            }
        }
        #[cfg(not(feature = "real-dock"))]
        {
            eprintln!(
                "plugin-dock-console: --include-real requires the `real-dock` feature — \
                 the real-window dock lane is not compiled into this build"
            );
            return ExitCode::from(2);
        }
    }
    if emit(options.out.as_ref(), &output) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Selector predicate — OR within a flag, AND across flags.
fn cell_selected(options: &Options, case: &str, scenario: &str, driver: ScenarioDriver) -> bool {
    (options.cases.is_empty() || options.cases.iter().any(|v| v.as_str() == case))
        && (options.scenarios.is_empty()
            || options.scenarios.iter().any(|v| v.as_str() == scenario))
        && (options.drivers.is_empty() || options.drivers.iter().any(|v| driver_name(driver) == *v))
}

/// `--run` selector validation — every refusal is fail-closed:
/// real-lane selectors first (the guidance names the lane that owns
/// real execution; the console never builds a second executor),
/// then unknown values, then combinations matching zero headless
/// cells.
fn validate_run_selectors(options: &Options) -> Result<(), String> {
    let real_driver = driver_name(real_catalog::DRIVER);
    if options.cases.iter().any(|value| {
        real_catalog::REAL_CATALOG
            .iter()
            .any(|row| row.case == value.as_str())
    }) || options
        .scenarios
        .iter()
        .any(|value| value == real_catalog::SCENARIO)
        || options.drivers.iter().any(|value| value == &real_driver)
    {
        return Err(
            "real-window dock cases are not executable through this console — use the \
             opt-in `plugin-dock-real-scenario` lane (`mise run \
             test-ui-plugin-dock-real`)"
                .to_string(),
        );
    }
    // Every selector value must name at least one headless catalog
    // cell — a typo is a usage error, not an empty run.
    for (flag, values, field) in [
        ("--case", &options.cases, "case"),
        ("--scenario", &options.scenarios, "scenario"),
        ("--driver", &options.drivers, "driver"),
    ] {
        for value in values {
            let known = HEADLESS_MATRIX.cells().any(|(entry, binding)| match field {
                "case" => entry.case == value.as_str(),
                "scenario" => entry.scenario == value.as_str(),
                _ => driver_name(binding.driver) == *value,
            });
            if !known {
                return Err(format!("unknown {flag} `{value}`"));
            }
        }
    }
    let selected = HEADLESS_MATRIX
        .cells()
        .filter(|(entry, binding)| {
            cell_selected(options, entry.case, entry.scenario, binding.driver)
        })
        .count();
    if selected == 0 {
        return Err(
            "no headless case matches the selector combination — nothing was executed".to_string(),
        );
    }
    Ok(())
}

/// `--run` — selector validation (fail-closed), then catalog-order
/// execution of the selected headless cells.
fn run(options: &Options) -> ExitCode {
    if let Err(message) = validate_run_selectors(options) {
        eprintln!("plugin-dock-console: {message}");
        return ExitCode::from(2);
    }
    let environment = LaneEnvironment {
        repo_sha: repo_sha(),
        plugin_path: env::var_os(PLUGIN_PATH_ENV).map(PathBuf::from),
    };
    let reports = run_selected(&environment, |entry, driver| {
        cell_selected(options, entry.case, entry.scenario, driver)
    });
    let output = render_jsonl(
        RUN_ID,
        CONSOLE_LANE,
        environment.repo_sha.as_deref(),
        &reports,
    );
    if !emit(options.out.as_ref(), &output) {
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

/// Write `content` to `--out` or stdout — same contract as the lane
/// binaries.
fn write_output(out: Option<&PathBuf>, content: &str) -> Result<(), String> {
    if let Some(path) = out {
        std::fs::write(path, content)
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(content.as_bytes())
            .and_then(|()| stdout.flush())
            .map_err(|error| format!("cannot write stdout: {error}"))
    }
}

/// Emit `content`, reporting write failures on stderr — `false` on
/// failure (caller maps to exit 1).
fn emit(out: Option<&PathBuf>, content: &str) -> bool {
    match write_output(out, content) {
        Ok(()) => true,
        Err(message) => {
            eprintln!("plugin-dock-console: {message}");
            false
        }
    }
}
