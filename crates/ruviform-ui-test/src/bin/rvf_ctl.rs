//! `rvf-ctl` — the ruviform verification control CLI (Issue #805).
//!
//! The agent-facing control entry for the repo's drive surfaces: a
//! thin CLI over the `rvf.uitest.v1` client ([`TestDriver`]) that
//! later slices extend with drive/evidence subcommands. This slice
//! ships `doctor` — the read-only health probe the `verify-ruviform`
//! skill's Doctor section calls.
//!
//! `doctor` resolves the app-under-test binary (default: the
//! `dummy_app` sibling of this binary in the same target dir), spawns
//! it through [`TestDriver::spawn_stdio`] or [`TestDriver::spawn_tcp`],
//! runs the `test.hello` + `test.capabilities` handshake, requests a
//! graceful `test.shutdown`, confirms the child exits, and prints one
//! structured JSON report. It is read-only: no fixture, clock, fault,
//! or window op is ever sent.
//!
//! Exit code: `0` when every check passes (or a `--dry-run` plan was
//! printed); `1` when the doctor could not verify — the report's
//! `verdict` is `fail` for a broken spawn/handshake/shutdown/exit
//! path and `not_run` + `environment_blocked` when the AUT binary is
//! missing; `2` on CLI misuse. `--help` prints usage and exits `0`.
//!
//! **RT contract:** non-RT only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ruviform_ui_test::{DriverError, ShutdownMode, TestDriver};
use serde_json::{Value, json};

/// Lane label written into reports — distinguishes `rvf-ctl doctor`
/// output from lane-runner evidence.
const DOCTOR_LANE: &str = "rvf-ctl-doctor";

const USAGE: &str = "\
rvf-ctl — the ruviform verification control CLI (Issue #805)

usage:
  rvf-ctl doctor [--app <path>] [--transport stdio|tcp] [--dry-run] [--out <path>]
  rvf-ctl --help | -h

subcommands:
  doctor   spawn the app-under-test over rvf.uitest.v1, run the
           test.hello + test.capabilities handshake, request a graceful
           test.shutdown, confirm the child exits, and print one JSON
           health report (read-only — no mutating op is sent)

options (doctor):
  --app <path>        AUT binary to drive; default: the `dummy_app`
                      sibling of this binary (same target dir)
  --transport <t>     `stdio` (default) or `tcp` — loopback, the driver
                      owns the ephemeral listener + run-bound secret
  --dry-run           print the execution plan JSON and exit — spawns
                      nothing, never touches the AUT
  --out <path>        write the JSON report to <path> instead of stdout
  --help, -h          this help

exit code:
  0  doctor passed (spawn + handshake + capabilities + graceful
     shutdown + child exit all verified), or a plan was printed
  1  doctor could not verify — report.verdict is `fail` for a broken
     spawn/handshake/capabilities/shutdown/exit path, `not_run` +
     environment_blocked when the AUT binary is missing
  2  CLI misuse (unknown subcommand/flag, missing value)

environment:
  The AUT side is fail-closed: it serves only when the driver opts it
  in via RVF_TEST_CONTROL (the driver sets it). RVF_TEST_DISABLE on
  this host refuses every serve path.

next steps when doctor fails:
  AUT binary missing → cargo build -p ruviform-ui-test --bin dummy_app
  handshake refused  → read the AUT stderr above: RVF_TEST_DISABLE set,
                       --fail-init, or an app that exits before serve
  tcp accept timeout → the AUT never dialed — check for a stall before
                       serve
  everything green   → drive it: mise run test-ui-plugin-dock-console
                       or read .agents/skills/verify-ruviform/SKILL.md";

/// Transport selection for the control channel.
#[derive(Clone, Copy)]
enum Transport {
    /// Piped stdin/stdout (default).
    Stdio,
    /// Loopback TCP with a run-bound secret.
    Tcp,
}

/// Parsed `doctor` options.
struct Options {
    /// `--app <path>`; `None` resolves to the sibling `dummy_app`.
    app: Option<PathBuf>,
    /// `--transport` selection.
    transport: Transport,
    /// `--dry-run` — plan only.
    dry_run: bool,
    /// `--out <path>`; `None` writes to stdout.
    out: Option<PathBuf>,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(sub) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match sub.as_str() {
        "--help" | "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        "doctor" => {
            let Some(options) = parse_doctor_args(args) else {
                return ExitCode::from(2);
            };
            doctor(&options)
        }
        _ => {
            eprintln!("rvf-ctl: unknown argument `{sub}`\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Thin `env::args` parse matching the sibling lane binaries —
/// `None` reports the error to stderr and yields exit 2.
fn parse_doctor_args(args: impl Iterator<Item = String>) -> Option<Options> {
    let mut args = args;
    let mut options = Options {
        app: None,
        transport: Transport::Stdio,
        dry_run: false,
        out: None,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--app" => {
                let Some(value) = args.next() else {
                    eprintln!("rvf-ctl: --app requires a value");
                    return None;
                };
                options.app = Some(PathBuf::from(value));
            }
            "--transport" => {
                let Some(value) = args.next() else {
                    eprintln!("rvf-ctl: --transport requires a value");
                    return None;
                };
                options.transport = match value.as_str() {
                    "stdio" => Transport::Stdio,
                    "tcp" => Transport::Tcp,
                    other => {
                        eprintln!("rvf-ctl: unknown transport `{other}` (stdio|tcp)");
                        return None;
                    }
                };
            }
            "--dry-run" => options.dry_run = true,
            "--out" => {
                let Some(value) = args.next() else {
                    eprintln!("rvf-ctl: --out requires a value");
                    return None;
                };
                options.out = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => {
                eprintln!("rvf-ctl: unknown argument `{arg}`");
                return None;
            }
        }
    }
    Some(options)
}

/// The default AUT: the `dummy_app` binary next to this one — the
/// same `target/<profile>/` dir `cargo build -p ruviform-ui-test
/// --bins` produces. Overridable with `--app` for other AUTs.
fn app_path(options: &Options) -> PathBuf {
    options.app.clone().unwrap_or_else(|| {
        let exe_name = format!("dummy_app{}", std::env::consts::EXE_SUFFIX);
        std::env::current_exe().map_or_else(
            |_| PathBuf::from(&exe_name),
            |exe| {
                exe.parent()
                    .map_or_else(|| PathBuf::from(&exe_name), |dir| dir.join(&exe_name))
            },
        )
    })
}

/// The repo commit: `GITHUB_SHA` (CI) → `git rev-parse HEAD` →
/// `None` recorded as `null` — never guessed (same convention as the
/// plugin-dock JSONL renderer).
fn repo_sha() -> Option<String> {
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

/// One `checks[]` row — `verdict` uses the shared vocabulary.
fn check(id: &str, verdict: &str, detail: Option<String>) -> Value {
    let mut c = json!({ "id": id, "verdict": verdict });
    if let Some(d) = detail {
        c["detail"] = json!(d);
    }
    c
}

/// The failure record plus the fix the operator should try next.
fn failure(stage: &str, detail: &str) -> Value {
    let next_step = match stage {
        "spawn" => {
            "rebuild the AUT — cargo build -p ruviform-ui-test --bin dummy_app \
             (a custom AUT needs --app <path>)"
        }
        "handshake" => {
            "read the AUT stderr above — refusals include RVF_TEST_DISABLE set, \
             --fail-init passed, or an app that exits before serve; for tcp the \
             accept bound also expires when the AUT stalls before dialing"
        }
        "capabilities" => {
            "the session is live but test.capabilities failed — compare \
             welcome.v against the crate's PROTOCOL_VERSION and read the \
             agent log on stderr"
        }
        "shutdown" => {
            "test.shutdown was not accepted — check for a stalled host loop \
             (--stall after-serve) or a driver-gone cleanup on stderr"
        }
        _ => {
            "the child outlived test.shutdown — the driver already killed and \
             reaped it; check for a stall fixture in the AUT"
        }
    };
    json!({
        "stage": stage,
        "detail": detail,
        "next_step": next_step,
    })
}

/// Writes the report as pretty JSON to `--out` or stdout. A write
/// failure is reported to stderr and downgrades the exit to `1`.
fn emit(options: &Options, report: &Value, code: ExitCode) -> ExitCode {
    let body = serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string());
    if let Some(path) = &options.out {
        match std::fs::write(path, format!("{body}\n")) {
            Ok(()) => code,
            Err(e) => {
                eprintln!("rvf-ctl: cannot write {}: {e}", path.display());
                ExitCode::FAILURE
            }
        }
    } else {
        let mut out = std::io::stdout().lock();
        match writeln!(out, "{body}") {
            Ok(()) => code,
            Err(e) => {
                eprintln!("rvf-ctl: stdout write failed: {e}");
                ExitCode::FAILURE
            }
        }
    }
}

/// `doctor` — spawn → `test.hello` + `test.capabilities` → graceful
/// `test.shutdown` → child-exit check, emitted as one JSON report.
fn doctor(options: &Options) -> ExitCode {
    let app = app_path(options);
    let transport_name = match options.transport {
        Transport::Stdio => "stdio",
        Transport::Tcp => "tcp",
    };

    if options.dry_run {
        return emit(
            options,
            &doctor_plan(&app, transport_name),
            ExitCode::SUCCESS,
        );
    }

    let mut report = json!({
        "type": "doctor",
        "lane": DOCTOR_LANE,
        "verdict": "not_run",
        "transport": transport_name,
        "app": app.display().to_string(),
        "repo": { "sha": repo_sha() },
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "environment_kind": Value::Null,
        },
    });
    let mut checks: Vec<Value> = Vec::new();
    let mut error = Value::Null;

    // The environment itself is missing — honest not_run, fail-closed.
    if !app.is_file() {
        checks.push(check(
            "spawn",
            "not_run",
            Some(format!("AUT binary not found: {}", app.display())),
        ));
        report["checks"] = json!(checks);
        report["error"] = json!({
            "stage": "spawn",
            "environment_blocked": true,
            "detail": format!("AUT binary not found: {}", app.display()),
            "next_step": "cargo build -p ruviform-ui-test --bin dummy_app (or pass --app <path>)",
        });
        eprintln!("rvf-ctl: AUT binary not found: {}", app.display());
        eprintln!(
            "next step: cargo build -p ruviform-ui-test --bin dummy_app (or pass --app <path>)"
        );
        return emit(options, &report, ExitCode::FAILURE);
    }

    // spawn + handshake (the driver performs test.hello on connect).
    let mut cmd = Command::new(&app);
    let spawned = match options.transport {
        Transport::Stdio => TestDriver::spawn_stdio(&mut cmd),
        Transport::Tcp => TestDriver::spawn_tcp(&mut cmd),
    };
    let mut driver = match spawned {
        Ok(d) => {
            checks.push(check("spawn", "pass", None));
            checks.push(check("handshake", "pass", None));
            d
        }
        Err(e) => {
            // `Io` means the process or channel never came up;
            // Handshake/Timeout mean the AUT answered badly or never
            // answered. `Disconnected` is still a handshake-stage
            // refusal (the channel died at hello).
            let spawn_failed = matches!(e, DriverError::Io(_));
            let stage = if spawn_failed { "spawn" } else { "handshake" };
            checks.push(check(
                "spawn",
                if spawn_failed { "fail" } else { "pass" },
                spawn_failed.then(|| e.to_string()),
            ));
            checks.push(check(
                "handshake",
                if spawn_failed { "not_run" } else { "fail" },
                (!spawn_failed).then(|| e.to_string()),
            ));
            checks.push(check("capabilities", "not_run", None));
            checks.push(check("shutdown", "not_run", None));
            checks.push(check("child-exit", "not_run", None));
            let failure = failure(stage, &e.to_string());
            eprintln!("rvf-ctl: {stage} failed: {e}");
            eprintln!(
                "next step: {}",
                failure["next_step"].as_str().unwrap_or_default()
            );
            report["checks"] = json!(checks);
            report["verdict"] = json!("fail");
            report["error"] = failure;
            return emit(options, &report, ExitCode::FAILURE);
        }
    };

    // `driver` drops at scope end: its `Drop` kills and reaps a
    // still-running child — never an orphan.
    let live = run_session(&mut driver, &mut report, &mut checks, &mut error);

    report["checks"] = json!(checks);
    report["verdict"] = json!(if live { "pass" } else { "fail" });
    if !error.is_null() {
        report["error"] = error;
    }
    emit(
        options,
        &report,
        if live {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        },
    )
}

/// The `--dry-run` plan: what would be spawned and which ops would be
/// sent — spawns nothing, never touches the AUT.
fn doctor_plan(app: &Path, transport_name: &str) -> Value {
    let mut plan = json!({
        "type": "doctor-plan",
        "lane": DOCTOR_LANE,
        "dry_run": true,
        "transport": transport_name,
        "app": app.display().to_string(),
        "app_exists": app.is_file(),
        "read_only": true,
        "ops": ["test.hello", "test.capabilities", "test.shutdown"],
        "steps": [
            "spawn the AUT with RVF_TEST_CONTROL=<transport> (piped stdio, or loopback TCP with a run-bound secret)",
            "test.hello → Welcome (v, session_id, run_id, app, ruviform_revision, backend, capabilities, windows, os, arch)",
            "test.capabilities → declared capability set + session_id",
            "test.shutdown graceful → CleanupReport",
            "await the child exit status",
        ],
    });
    if !app.is_file() {
        plan["next_step"] =
            json!("AUT binary not found — cargo build -p ruviform-ui-test --bin dummy_app");
    }
    plan
}

/// The live session after a successful handshake: welcome →
/// capabilities → graceful shutdown → child-exit. Returns `false`
/// when any stage fails; `error` records the first failure.
fn run_session(
    driver: &mut TestDriver,
    report: &mut Value,
    checks: &mut Vec<Value>,
    error: &mut Value,
) -> bool {
    let mut live = true;

    report["welcome"] = serde_json::to_value(driver.welcome()).unwrap_or_default();
    report["environment"]["environment_kind"] = match driver.welcome().backend.name.as_str() {
        "headless-fake" => json!("deterministic-backend"),
        _ => Value::Null,
    };
    match driver.capabilities() {
        Ok(caps) => {
            checks.push(check("capabilities", "pass", None));
            report["capabilities"] = caps;
        }
        Err(e) => {
            live = false;
            checks.push(check("capabilities", "fail", Some(e.to_string())));
            *error = failure("capabilities", &e.to_string());
            eprintln!("rvf-ctl: capabilities failed: {e}");
            eprintln!(
                "next step: {}",
                error["next_step"].as_str().unwrap_or_default()
            );
        }
    }

    // Graceful teardown — attempted whenever a session exists
    // (cleanup is part of the proof, not optional on failure).
    match driver.shutdown(ShutdownMode::Graceful) {
        Ok(cleanup) => {
            checks.push(check("shutdown", "pass", None));
            report["cleanup"] = serde_json::to_value(&cleanup).unwrap_or_default();
        }
        Err(e) => {
            live = false;
            checks.push(check("shutdown", "fail", Some(e.to_string())));
            if error.is_null() {
                *error = failure("shutdown", &e.to_string());
            }
            eprintln!("rvf-ctl: shutdown failed: {e}");
        }
    }

    // Child-exit observation — independent of the shutdown reply.
    match driver.try_exit_status() {
        Ok(Some(status)) => {
            report["child_exit"] = json!({ "success": status.success(), "code": status.code() });
            if status.success() {
                checks.push(check("child-exit", "pass", None));
            } else {
                live = false;
                checks.push(check(
                    "child-exit",
                    "fail",
                    Some(format!("child exited {status}")),
                ));
                if error.is_null() {
                    *error = failure("child-exit", &format!("child exited {status}"));
                }
            }
        }
        Ok(None) => {
            live = false;
            let _ = driver.kill();
            report["child_exit"] = json!({
                "success": false,
                "detail": "child still running after test.shutdown — killed and reaped by the driver",
            });
            checks.push(check(
                "child-exit",
                "fail",
                Some("child still running after test.shutdown — killed".to_string()),
            ));
            if error.is_null() {
                *error = failure("child-exit", "child still running after test.shutdown");
            }
        }
        Err(e) => {
            live = false;
            checks.push(check("child-exit", "fail", Some(e.to_string())));
            if error.is_null() {
                *error = failure("child-exit", &e.to_string());
            }
        }
    }
    live
}
