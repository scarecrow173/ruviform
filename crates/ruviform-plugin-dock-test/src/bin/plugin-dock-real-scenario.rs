//! `plugin-dock-real-scenario` — the Issue #631/#630 real-window
//! plugin-dock lane entry (feature `real-dock`).
//!
//! Runs every catalog case of ONE plugin format inside one
//! `RealOsGuiTest` session — real OS windows on the interactive
//! desktop — and emits the reports as JSONL in the same shape as
//! `plugin-dock-scenario` (shared `jsonl` renderer): one `run`
//! header line followed by one `case` line per [`ScenarioReport`].
//! Output goes to `--out <path>` or stdout.
//!
//! `--format vst3|clap` (default `vst3`) selects the Issue #630 AC4
//! leg: `vst3` runs the canonical `rf631.plugin-dock-real` catalog
//! against `ruviform_plugin_test_vst3.dll`; `clap` runs the
//! `rf630.plugin-dock-real-clap` catalog against
//! `ruviform_plugin_test_clap.dll`'s `test_editor` — a separate run
//! id, scenario id, and JSONL evidence stream, never interleaved.
//! Each format reads its own explicit-path env var
//! (`RUVIFORM_PLUGIN_DOCK_TEST_VST3` /
//! `RUVIFORM_PLUGIN_DOCK_TEST_CLAP`, see [`PluginFormat::path_env`]).
//!
//! `--pixel-artifacts <dir>` is opt-in: the `real-capture`,
//! `real-dcomp-pixel`, `real-dcomp-foreground`,
//! `real-dcomp-present`, `real-dcomp-uncovered`, and
//! `real-dcomp-bandescape` cases then write
//! each captured frame as a
//! PNG under `<dir>` (created on demand), recording
//! `artifact: {path, sha256}` alongside the canonical
//! `capture_sha256` pixel digest. Without it the cases record
//! digests and statistics only — the canonical lane never writes
//! artifacts.
//!
//! `--blocked-probe` is the headless fail-closed proof (Issue #631
//! real-row slice): it generates the same fail-closed report
//! mapping a `RealOsGuiTest::run` startup failure produces —
//! WITHOUT opening a session, so the proof is deterministic on any
//! host — self-
//! checks that every catalog row is `NotRun` +
//! `observed.environment_blocked` and none claims
//! `environment_kind=real-window`, and exits `1` on any violation.
//! The reports' detail says plainly that no session start was
//! attempted: they evidence the fail-closed *mapping*, never an
//! observed headless run. The probe is format-aware: under
//! `--format clap` it proves the CLAP catalog's fail-closed shape.
//!
//! Exit code: `0` unless any case verdict is `Fail` (or the
//! blocked-probe self-check finds a violation); `NotRun`/
//! `Unsupported` are honest recorded outcomes — a headless host
//! records `NotRun` + `observed=environment_blocked` for every case
//! (fail-closed), which must not red the lane. A `Fail` verdict —
//! e.g. `real-dcomp-pixel` under the known unresolved GPUI `DComp`
//! topmost occlusion — exits `1` by design: that is the honest
//! regression signal, not a lane defect.
//!
//! **RT contract:** non-RT only — opens real OS windows (except
//! `--blocked-probe`, which opens none).

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use ruviform_plugin_dock_test::LaneEnvironment;
use ruviform_plugin_dock_test::jsonl::{render_jsonl_with_path_env, repo_sha};
use ruviform_plugin_dock_test::real_dock::{
    PluginFormat, blocked_probe_violations_with_format, blocked_probe_with_format,
    run_all_with_format,
};
use ruviform_ui_test::ScenarioVerdict;

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut pixel_artifacts: Option<PathBuf> = None;
    let mut blocked_probe_mode = false;
    let mut format = PluginFormat::Vst3;
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
            "--format" => match args.next().as_deref() {
                Some("vst3") => format = PluginFormat::Vst3,
                Some("clap") => format = PluginFormat::Clap,
                Some(other) => {
                    eprintln!(
                        "plugin-dock-real-scenario: unknown format `{other}` \
                             (expected vst3|clap)"
                    );
                    return ExitCode::from(2);
                }
                None => {
                    eprintln!("plugin-dock-real-scenario: --format requires vst3|clap");
                    return ExitCode::from(2);
                }
            },
            "--blocked-probe" => {
                blocked_probe_mode = true;
            }
            other => {
                eprintln!("plugin-dock-real-scenario: unknown argument `{other}`");
                return ExitCode::from(2);
            }
        }
    }

    let environment = LaneEnvironment {
        repo_sha: repo_sha(),
        plugin_path: env::var_os(format.path_env()).map(PathBuf::from),
    };
    let (reports, lane) = if blocked_probe_mode {
        (
            blocked_probe_with_format(&environment, format),
            "test-ui-plugin-dock-real-headless",
        )
    } else {
        (
            run_all_with_format(&environment, pixel_artifacts.as_deref(), format),
            "test-ui-plugin-dock-real",
        )
    };
    let jsonl = render_jsonl_with_path_env(
        format.run_id(),
        lane,
        environment.repo_sha.as_deref(),
        format.path_env(),
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

    if blocked_probe_mode {
        let violations = blocked_probe_violations_with_format(&reports, format);
        if !violations.is_empty() {
            for violation in &violations {
                eprintln!("plugin-dock-real-scenario: blocked-probe violation: {violation}");
            }
            return ExitCode::FAILURE;
        }
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
