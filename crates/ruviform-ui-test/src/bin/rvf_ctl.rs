//! `rvf-ctl` — the ruviform verification control CLI (Issue #805).
//!
//! The agent-facing control entry for the repo's drive surfaces: a
//! thin CLI over the `rvf.uitest.v1` client ([`TestDriver`]). It
//! ships two subcommands:
//!
//! - `doctor` — the read-only health probe the `verify-ruviform`
//!   skill's Doctor section calls. It resolves the app-under-test
//!   binary (default: the `dummy_app` sibling of this binary in the
//!   same target dir), spawns it through [`TestDriver::spawn_stdio`]
//!   or [`TestDriver::spawn_tcp`], runs the `test.hello` +
//!   `test.capabilities` handshake, requests a graceful
//!   `test.shutdown`, confirms the child exits, and prints one
//!   structured JSON report. It never sends a fixture/clock/fault/
//!   window op.
//! - `drive` — the composable drive surface the skill's Drive
//!   section calls. One invocation is one session: spawn →
//!   `test.hello` → the ordered step list → graceful `test.shutdown`
//!   → child-exit. Each step is one closed-set op written
//!   `<op>[=<json-params>]` — `capabilities`, `observe.windows`,
//!   `observe.receipts`, `observe.screenshot`, `notices`,
//!   `fixture.prepare`, `clock.advance`, `fault.set`, `fault.clear`,
//!   `window.request_close` (a `test.` prefix is accepted).
//!
//! `drive` steps run in argv order and may chain: `"window":"last"`
//! resolves to the most recently observed or created window id
//! (welcome → fixture receipt → `observe.windows`). Per-step
//! verdicts use the shared vocabulary — an `unavailable` protocol
//! answer records `unsupported` (capability-gated, never a fake
//! pass), a refused op records `fail`, and a step that cannot run
//! (selector unresolvable, channel already dead) records `not_run`.
//!
//! Exit code: `0` when every check and step passed (or a `--dry-run`
//! plan was printed); `1` when the command could not verify — the
//! report's `verdict` is `fail` for a broken spawn/handshake/op/
//! shutdown/exit path or an unexecuted step, and `not_run` +
//! `environment_blocked` when the AUT binary is missing; `2` on CLI
//! misuse. `--help` prints usage and exits `0`.
//!
//! **RT contract:** non-RT only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ruviform_ui_test::{ClockDomain, DriverError, FaultKind, ShutdownMode, TestDriver};
use serde_json::{Value, json};

/// Lane label written into reports — distinguishes `rvf-ctl doctor`
/// output from lane-runner evidence.
const DOCTOR_LANE: &str = "rvf-ctl-doctor";
/// Lane label written into `drive` reports.
const DRIVE_LANE: &str = "rvf-ctl-drive";

const USAGE: &str = "\
rvf-ctl — the ruviform verification control CLI (Issue #805)

usage:
  rvf-ctl doctor [--app <path>] [--app-args \"<args>\"] [--transport stdio|tcp] [--dry-run] [--out <path>]
  rvf-ctl drive  [--app <path>] [--app-args \"<args>\"] [--transport stdio|tcp] [--dry-run] [--out <path>] <step>...
  rvf-ctl --help | -h

subcommands:
  doctor   spawn the app-under-test over rvf.uitest.v1, run the
           test.hello + test.capabilities handshake, request a graceful
           test.shutdown, confirm the child exits, and print one JSON
           health report (read-only — no mutating op is sent)
  drive    one session per invocation: spawn → test.hello → the
           ordered <step> list → graceful test.shutdown → child-exit.
           Each <step> is `<op>[=<json params>]` from the closed op
           set (the `test.` prefix is optional):

             capabilities
             observe.windows
             observe.receipts={\"since\":0,\"limit\":4096}   (both optional)
             observe.screenshot={\"window\":<id|\"last\">}
             notices
             fixture.prepare={\"name\":\"<fixture>\"[,\"seed\":<u64>][,\"params\":{...}]}
             clock.advance={\"domain\":\"<domain>\",\"ticks\":<u64>}
             fault.set={\"kind\":\"<kind>\",\"window\":<id|\"last\">}
             fault.clear={\"kind\":\"<kind>\",\"window\":<id|\"last\">}
             window.request_close={\"window\":<id|\"last\">}

           <id|\"last\">: a window id as u64, or the string `last`
           for the most recently observed/created window (welcome →
           fixture receipt → observe.windows). Domains and fault kinds
           are validated against the closed enums before spawn — an
           unknown value is exit 2, never sent on the wire. Steps run
           in argv order; a step the agent refuses records `fail`
           (or `unsupported` when the answer is `unavailable`), a step
           that cannot run records `not_run`, and the session still
           ends with a graceful shutdown + child-exit check.

options (both subcommands):
  --app <path>        AUT binary to drive; default: the `dummy_app`
                      sibling of this binary (same target dir)
  --app-args \"<s>\"   extra argv passed to the spawned AUT, split on
                      whitespace (for example \"--windows 2\" or
                      \"--fail-init\" — see the AUT's own flags)
  --transport <t>     `stdio` (default) or `tcp` — loopback, the driver
                      owns the ephemeral listener + run-bound secret
  --dry-run           print the execution plan JSON and exit — spawns
                      nothing, never touches the AUT
  --out <path>        write the JSON report to <path> instead of stdout
  --help, -h          this help

exit code:
  0  every check and step passed (unsupported outcomes are honest
     answers, not failures), or a plan was printed
  1  the run could not verify — report.verdict is `fail` for a broken
     spawn/handshake/step/shutdown/exit path or a step left `not_run`,
     `not_run` + environment_blocked when the AUT binary is missing
  2  CLI misuse (unknown subcommand/flag/step, missing value, bad
     step JSON, unknown clock domain or fault kind)

environment:
  The AUT side is fail-closed: it serves only when the driver opts it
  in via RVF_TEST_CONTROL (the driver sets it). RVF_TEST_DISABLE on
  this host refuses every serve path.

next steps when a run fails:
  AUT binary missing → cargo build -p ruviform-ui-test --bin dummy_app
  handshake refused  → read the AUT stderr above: RVF_TEST_DISABLE set,
                       --fail-init, or an app that exits before serve
  tcp accept timeout → the AUT never dialed — check for a stall before
                       serve
  step `unsupported` → capability gated — compare welcome.capabilities
  step `fail`        → read the step's error detail; params must match
                       the op schema above
  everything green   → record evidence and read
                       .agents/skills/verify-ruviform/SKILL.md";

/// Transport selection for the control channel.
#[derive(Clone, Copy)]
enum Transport {
    /// Piped stdin/stdout (default).
    Stdio,
    /// Loopback TCP with a run-bound secret.
    Tcp,
}

/// Options shared by `doctor` and `drive`.
struct Options {
    /// `--app <path>`; `None` resolves to the sibling `dummy_app`.
    app: Option<PathBuf>,
    /// `--app-args "<s>"` — extra AUT argv, split on whitespace.
    app_args: Vec<String>,
    /// `--transport` selection.
    transport: Transport,
    /// `--dry-run` — plan only.
    dry_run: bool,
    /// `--out <path>`; `None` writes to stdout.
    out: Option<PathBuf>,
}

/// One validated `drive` step — `<op>[=<json params>]` held as the
/// canonical wire op name plus its params object. `window` params may
/// hold the string `"last"`, resolved against session state at run
/// time; every other shape is checked here at parse time.
struct Step {
    /// Canonical op name on the wire (`test.observe.windows`, …).
    op: &'static str,
    /// The step's params object (`{}` when the op takes none).
    params: Value,
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
            let Some((options, rest)) = parse_shared_args(args) else {
                return ExitCode::from(2);
            };
            if !rest.is_empty() {
                eprintln!("rvf-ctl: doctor takes no steps (got `{}`)", rest[0]);
                return ExitCode::from(2);
            }
            doctor(&options)
        }
        "drive" => {
            let Some((options, rest)) = parse_shared_args(args) else {
                return ExitCode::from(2);
            };
            if rest.is_empty() {
                eprintln!("rvf-ctl: drive requires at least one <step>\n{USAGE}");
                return ExitCode::from(2);
            }
            let mut steps = Vec::with_capacity(rest.len());
            for (i, token) in rest.iter().enumerate() {
                match parse_step(token) {
                    Ok(step) => steps.push(step),
                    Err(message) => {
                        eprintln!("rvf-ctl: step {} `{token}`: {message}", i + 1);
                        return ExitCode::from(2);
                    }
                }
            }
            drive(&options, &steps)
        }
        _ => {
            eprintln!("rvf-ctl: unknown argument `{sub}`\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Thin `env::args` parse matching the sibling lane binaries —
/// `None` reports the error to stderr and yields exit 2. Returns the
/// shared options plus the unconsumed positional tokens (drive's
/// step list).
fn parse_shared_args(args: impl Iterator<Item = String>) -> Option<(Options, Vec<String>)> {
    let mut args = args;
    let mut options = Options {
        app: None,
        app_args: Vec::new(),
        transport: Transport::Stdio,
        dry_run: false,
        out: None,
    };
    let mut positional = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--app" => {
                let Some(value) = args.next() else {
                    eprintln!("rvf-ctl: --app requires a value");
                    return None;
                };
                options.app = Some(PathBuf::from(value));
            }
            "--app-args" => {
                let Some(value) = args.next() else {
                    eprintln!("rvf-ctl: --app-args requires a value");
                    return None;
                };
                options
                    .app_args
                    .extend(value.split_whitespace().map(str::to_string));
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
            _ if arg.starts_with('-') => {
                eprintln!("rvf-ctl: unknown argument `{arg}`");
                return None;
            }
            _ => positional.push(arg),
        }
    }
    Some((options, positional))
}

/// Maps a step name to its canonical wire op — the `test.` prefix is
/// optional on the command line. `None` is an unknown op.
fn step_op(name: &str) -> Option<&'static str> {
    use ruviform_ui_test::ops;
    Some(match name.strip_prefix("test.").unwrap_or(name) {
        "capabilities" => ops::CAPABILITIES,
        "observe.windows" => ops::OBSERVE_WINDOWS,
        "observe.receipts" => ops::OBSERVE_RECEIPTS,
        "observe.screenshot" => ops::OBSERVE_SCREENSHOT,
        "notices" => ops::NOTICES,
        "fixture.prepare" => ops::FIXTURE_PREPARE,
        "clock.advance" => ops::CLOCK_ADVANCE,
        "fault.set" => ops::FAULT_SET,
        "fault.clear" => ops::FAULT_CLEAR,
        "window.request_close" => ops::WINDOW_REQUEST_CLOSE,
        _ => return None,
    })
}

/// Reads a required u64-or-`"last"` field — `last` is the session
/// selector resolved at run time, everything else is a parse error.
fn window_param(params: &Value, key: &str) -> Result<Value, String> {
    match params.get(key) {
        Some(v @ (Value::Number(_) | Value::String(_))) => {
            let ok = v.as_u64().is_some() || v.as_str() == Some("last");
            if ok {
                Ok(v.clone())
            } else {
                Err(format!("`{key}` must be a window id (u64) or \"last\""))
            }
        }
        Some(_) => Err(format!("`{key}` must be a window id (u64) or \"last\"")),
        None => Err(format!("`{key}` is required")),
    }
}

/// Rejects params keys the op does not know — a typo is CLI misuse
/// (exit 2), never silently dropped params.
fn check_keys(op: &str, params: &Value, known: &[&str]) -> Result<(), String> {
    if let Some(obj) = params.as_object() {
        for key in obj.keys() {
            if !known.contains(&key.as_str()) {
                return Err(format!(
                    "unknown param `{key}` for {op} (valid: {})",
                    known.join(", ")
                ));
            }
        }
    }
    Ok(())
}

/// Parses one `<op>[=<json params>]` step token — every validation
/// that can happen without a session happens here so a malformed
/// step is exit 2 before anything spawns.
fn parse_step(token: &str) -> Result<Step, String> {
    let (name, raw_params) = match token.split_once('=') {
        Some((name, raw)) => (name, Some(raw)),
        None => (token, None),
    };
    let op = step_op(name)
        .ok_or_else(|| format!("unknown op `{name}` — run rvf-ctl --help for the closed op set"))?;
    let params = match raw_params {
        None => json!({}),
        Some(raw) => {
            serde_json::from_str::<Value>(raw).map_err(|e| format!("bad JSON params: {e}"))?
        }
    };
    if !params.is_object() {
        return Err("step params must be a JSON object".to_string());
    }
    match op {
        // No-param ops: `=...` must be `{}` or absent.
        "test.capabilities" | "test.observe.windows" | "test.notices" => {
            if !params.as_object().is_none_or(serde_json::Map::is_empty) {
                return Err(format!("{op} takes no parameters"));
            }
        }
        "test.observe.receipts" => {
            check_keys(op, &params, &["since", "limit"])?;
            for key in ["since", "limit"] {
                if params.get(key).is_some_and(|v| v.as_u64().is_none()) {
                    return Err(format!("`{key}` must be a u64"));
                }
            }
        }
        "test.observe.screenshot" | "test.window.request_close" => {
            check_keys(op, &params, &["window"])?;
            window_param(&params, "window")?;
        }
        "test.fixture.prepare" => {
            check_keys(op, &params, &["name", "seed", "params"])?;
            match params.get("name").and_then(Value::as_str) {
                Some(n) if !n.is_empty() => {}
                Some(_) => return Err("`name` must be a non-empty fixture name".to_string()),
                None => return Err("`name` is required".to_string()),
            }
            if params.get("seed").is_some_and(|v| v.as_u64().is_none()) {
                return Err("`seed` must be a u64".to_string());
            }
            if params.get("params").is_some_and(|v| !v.is_object()) {
                return Err("`params` must be a JSON object".to_string());
            }
        }
        "test.clock.advance" => {
            check_keys(op, &params, &["domain", "ticks"])?;
            let Some(domain) = params.get("domain").and_then(Value::as_str) else {
                return Err("`domain` is required".to_string());
            };
            if ClockDomain::parse(domain).is_none() {
                return Err(format!(
                    "unknown clock domain `{domain}` (known: host_tick)"
                ));
            }
            match params.get("ticks").and_then(Value::as_u64) {
                Some(t) if t > 0 => {}
                Some(_) => return Err("`ticks` must be a positive u64".to_string()),
                None => return Err("`ticks` is required".to_string()),
            }
        }
        "test.fault.set" | "test.fault.clear" => {
            check_keys(op, &params, &["kind", "window"])?;
            let Some(kind) = params.get("kind").and_then(Value::as_str) else {
                return Err("`kind` is required".to_string());
            };
            if FaultKind::parse(kind).is_none() {
                return Err(format!(
                    "unknown fault kind `{kind}` (known: window_crash, ipc_disconnect)"
                ));
            }
            window_param(&params, "window")?;
        }
        _ => unreachable!("step_op maps only closed-set ops"),
    }
    Ok(Step { op, params })
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
        _ if stage.starts_with("op:") => {
            "read the step's error detail — check the step params against the \
             op schema (`rvf-ctl --help`) and the op against \
             welcome.capabilities (`unavailable` means the backend does not \
             declare it); a dead channel leaves later steps not_run"
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

/// The shared spawn prelude: resolve the AUT, honor `--dry-run`
/// through `plan`, report a missing binary as `not_run` +
/// `environment_blocked`, then spawn + handshake. Returns the live
/// driver, or the already-emitted exit decision as `Err(code)`.
fn spawn_session(
    options: &Options,
    report: &mut Value,
    checks: &mut Vec<Value>,
    plan: &Value,
) -> Result<TestDriver, ExitCode> {
    if options.dry_run {
        return Err(emit(options, plan, ExitCode::SUCCESS));
    }
    let app = PathBuf::from(report["app"].as_str().unwrap_or_default());
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
        return Err(emit(options, report, ExitCode::FAILURE));
    }

    // spawn + handshake (the driver performs test.hello on connect).
    let mut cmd = Command::new(&app);
    cmd.args(&options.app_args);
    let spawned = match options.transport {
        Transport::Stdio => TestDriver::spawn_stdio(&mut cmd),
        Transport::Tcp => TestDriver::spawn_tcp(&mut cmd),
    };
    match spawned {
        Ok(d) => {
            checks.push(check("spawn", "pass", None));
            checks.push(check("handshake", "pass", None));
            Ok(d)
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
            let failure = failure(stage, &e.to_string());
            eprintln!("rvf-ctl: {stage} failed: {e}");
            eprintln!(
                "next step: {}",
                failure["next_step"].as_str().unwrap_or_default()
            );
            report["checks"] = json!(checks);
            report["verdict"] = json!("fail");
            report["error"] = failure;
            Err(emit(options, report, ExitCode::FAILURE))
        }
    }
}

/// The shared teardown: graceful `test.shutdown` → `CleanupReport`,
/// then the child-exit observation (a still-running child is killed
/// and reaped). Returns `false` when either stage fails; `error`
/// records the first failure when not already set.
fn teardown(
    driver: &mut TestDriver,
    report: &mut Value,
    checks: &mut Vec<Value>,
    error: &mut Value,
) -> bool {
    let mut live = true;
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

/// `doctor` — spawn → `test.hello` + `test.capabilities` → graceful
/// `test.shutdown` → child-exit check, emitted as one JSON report.
fn doctor(options: &Options) -> ExitCode {
    let app = app_path(options);
    let transport_name = match options.transport {
        Transport::Stdio => "stdio",
        Transport::Tcp => "tcp",
    };
    let mut report = json!({
        "type": "doctor",
        "lane": DOCTOR_LANE,
        "verdict": "not_run",
        "transport": transport_name,
        "app": app.display().to_string(),
        "app_args": options.app_args,
        "repo": { "sha": repo_sha() },
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "environment_kind": Value::Null,
        },
    });
    let mut checks: Vec<Value> = Vec::new();
    let mut driver = match spawn_session(
        options,
        &mut report,
        &mut checks,
        &doctor_plan(&app, transport_name),
    ) {
        Ok(d) => d,
        Err(code) => return code,
    };

    // `driver` drops at scope end: its `Drop` kills and reaps a
    // still-running child — never an orphan.
    let mut error = Value::Null;
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

/// The `doctor --dry-run` plan: what would be spawned and which ops
/// would be sent — spawns nothing, never touches the AUT.
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
    report["environment"]["environment_kind"] =
        environment_kind(driver.welcome().backend.name.as_str());
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

    live &= teardown(driver, report, checks, error);
    live
}

/// Maps the AUT's declared backend name to the evidence
/// `environment_kind` — recorded only from a real negotiated
/// `Welcome`, never assumed (same honest-vocabulary convention as
/// the plugin-dock lanes).
fn environment_kind(backend: &str) -> Value {
    match backend {
        "headless-fake" => json!("deterministic-backend"),
        "gpui-windowed" => json!("real-window"),
        _ => Value::Null,
    }
}

/// `drive` — spawn → `test.hello` → the ordered step list → graceful
/// `test.shutdown` → child-exit, emitted as one JSON report.
fn drive(options: &Options, steps: &[Step]) -> ExitCode {
    let app = app_path(options);
    let transport_name = match options.transport {
        Transport::Stdio => "stdio",
        Transport::Tcp => "tcp",
    };
    let mut report = json!({
        "type": "drive",
        "lane": DRIVE_LANE,
        "verdict": "not_run",
        "transport": transport_name,
        "app": app.display().to_string(),
        "app_args": options.app_args,
        "repo": { "sha": repo_sha() },
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "environment_kind": Value::Null,
        },
    });
    let mut checks: Vec<Value> = Vec::new();
    let mut driver = match spawn_session(
        options,
        &mut report,
        &mut checks,
        &drive_plan(&app, transport_name, steps),
    ) {
        Ok(d) => d,
        Err(code) => return code,
    };

    report["welcome"] = serde_json::to_value(driver.welcome()).unwrap_or_default();
    report["environment"]["environment_kind"] =
        environment_kind(driver.welcome().backend.name.as_str());

    let mut error = Value::Null;
    let mut last_window = driver.welcome().windows.last().map(|w| w.id);
    let all_ok = run_steps(
        &mut driver,
        steps,
        &mut last_window,
        &mut report,
        &mut error,
    );

    let live = teardown(&mut driver, &mut report, &mut checks, &mut error) && all_ok;

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

/// Runs the validated steps against the live session — fills
/// `report["steps"]` with one record per step and returns whether
/// every step reached the wire honestly (`unsupported` is an
/// honest answer; `not_run`/`fail` are not). Async notices
/// buffered while an op ran attach to that step's record.
fn run_steps(
    driver: &mut TestDriver,
    steps: &[Step],
    last_window: &mut Option<u64>,
    report: &mut Value,
    error: &mut Value,
) -> bool {
    let mut step_records: Vec<Value> = Vec::with_capacity(steps.len());
    let mut channel_dead = false;
    let mut all_ok = true;
    for (i, step) in steps.iter().enumerate() {
        let mut rec = json!({ "seq": i + 1, "op": step.op, "params": step.params });
        if channel_dead {
            rec["verdict"] = json!("not_run");
            rec["detail"] = json!("control channel is already dead — session ended earlier");
            step_records.push(rec);
            continue;
        }
        match run_step(driver, step, last_window) {
            StepOutcome::Ok(reply) => {
                rec["verdict"] = json!("pass");
                rec["reply"] = reply;
            }
            StepOutcome::Unsupported(e) => {
                rec["verdict"] = json!("unsupported");
                rec["error"] = json!(e.to_string());
            }
            StepOutcome::NotRun(detail) => {
                rec["verdict"] = json!("not_run");
                rec["detail"] = json!(detail);
                all_ok = false;
                if error.is_null() {
                    *error = failure(&format!("op:{}", step.op), &detail);
                }
            }
            StepOutcome::Failed(e) => {
                rec["verdict"] = json!("fail");
                rec["error"] = json!(e.to_string());
                all_ok = false;
                if matches!(
                    e,
                    DriverError::Disconnected | DriverError::Io(_) | DriverError::Timeout
                ) {
                    channel_dead = true;
                }
                if error.is_null() {
                    *error = failure(&format!("op:{}", step.op), &e.to_string());
                }
                eprintln!("rvf-ctl: step {} `{}` failed: {e}", i + 1, step.op);
            }
        }
        let notices: Vec<Value> = driver
            .notifications()
            .into_iter()
            .filter_map(|v| v.get("notice").cloned())
            .collect();
        if !notices.is_empty() {
            rec["notices"] = json!(notices);
        }
        step_records.push(rec);
    }
    report["steps"] = json!(step_records);
    all_ok
}

/// The `drive --dry-run` plan: what would be spawned and the
/// validated step list — spawns nothing, never touches the AUT.
fn drive_plan(app: &Path, transport_name: &str, steps: &[Step]) -> Value {
    let mut plan = json!({
        "type": "drive-plan",
        "lane": DRIVE_LANE,
        "dry_run": true,
        "transport": transport_name,
        "app": app.display().to_string(),
        "app_exists": app.is_file(),
        "read_only": false,
        "session": [
            "spawn the AUT with RVF_TEST_CONTROL=<transport> (+ --app-args argv)",
            "test.hello → Welcome",
            "run each step in order — mutating ops change live AUT state",
            "test.shutdown graceful → CleanupReport",
            "await the child exit status",
        ],
        "steps": steps.iter().enumerate().map(|(i, s)| json!({
            "seq": i + 1,
            "op": s.op,
            "params": s.params,
        })).collect::<Vec<_>>(),
    });
    if !app.is_file() {
        plan["next_step"] =
            json!("AUT binary not found — cargo build -p ruviform-ui-test --bin dummy_app");
    }
    plan
}

/// One step's outcome — `unsupported` is a capability-gated
/// `unavailable` answer, `not_run` is a step that never reached the
/// wire, `failed` is every other refusal.
enum StepOutcome {
    /// The op answered `ok` — the reply payload.
    Ok(Value),
    /// The agent answered `unavailable` (capability gate or missing
    /// backend feature) — honest `unsupported`, not a failure.
    Unsupported(ruviform_ui::ErrorBody),
    /// The step could not be issued (selector unresolvable) — never
    /// reached the wire.
    NotRun(String),
    /// A protocol error or a transport/channel failure.
    Failed(DriverError),
}

/// Resolves a `"last"` window selector against session state — the
/// id is filled into `params[key]` or the step stays `not_run`. A
/// `key` absent from params needs no resolution.
fn resolve_window(params: &mut Value, key: &str, last_window: Option<u64>) -> Result<(), String> {
    if params.get(key).and_then(Value::as_str) == Some("last") {
        match last_window {
            Some(id) => {
                params[key] = json!(id);
                Ok(())
            }
            None => Err(format!(
                "`{key}` is \"last\" but no window has been observed yet — \
                 run observe.windows or a window-creating fixture first"
            )),
        }
    } else {
        Ok(())
    }
}

/// Executes one validated step against the live session and updates
/// `last_window` from the reply (fixture `applied.windows` /
/// `observe.windows` results feed later `"last"` selectors).
fn run_step(driver: &mut TestDriver, step: &Step, last_window: &mut Option<u64>) -> StepOutcome {
    use ruviform_ui_test::ops;
    let mut params = step.params.clone();
    if let Err(detail) = resolve_window(&mut params, "window", *last_window) {
        return StepOutcome::NotRun(detail);
    }
    let window = params.get("window").and_then(Value::as_u64).unwrap_or(0);
    let result = match step.op {
        ops::CAPABILITIES => driver.capabilities(),
        ops::OBSERVE_WINDOWS => driver.observe_windows().map(|windows| {
            *last_window = windows.last().map(|w| w.id).or(*last_window);
            json!({ "windows": windows })
        }),
        ops::OBSERVE_RECEIPTS => driver
            .observe_receipts(
                params.get("since").and_then(Value::as_u64).unwrap_or(0),
                params.get("limit").and_then(Value::as_u64).unwrap_or(4096),
            )
            .map(|page| serde_json::to_value(page).unwrap_or_default()),
        ops::OBSERVE_SCREENSHOT => driver.screenshot(window).map(|shot| {
            let decoded_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &shot.data_base64,
            )
            .map_or(0, |b| b.len());
            json!({
                "format": shot.format,
                "width": shot.width,
                "height": shot.height,
                "decoded_bytes": decoded_bytes,
            })
        }),
        ops::NOTICES => driver
            .notices()
            .map(|notices| json!({ "notices": notices })),
        ops::FIXTURE_PREPARE => driver
            .prepare_fixture(
                params["name"].as_str().unwrap_or_default(),
                params.get("seed").and_then(Value::as_u64),
                params.get("params").cloned().unwrap_or_else(|| json!({})),
            )
            .map(|receipt| {
                *last_window = receipt
                    .applied
                    .get("windows")
                    .and_then(Value::as_array)
                    .and_then(|w| w.last())
                    .and_then(Value::as_u64)
                    .or(*last_window);
                serde_json::to_value(receipt).unwrap_or_default()
            }),
        ops::CLOCK_ADVANCE => driver.advance_clock(
            ClockDomain::parse(params["domain"].as_str().unwrap_or_default())
                .unwrap_or(ClockDomain::HostTick),
            params["ticks"].as_u64().unwrap_or(1),
        ),
        ops::FAULT_SET => driver.set_fault(
            FaultKind::parse(params["kind"].as_str().unwrap_or_default())
                .unwrap_or(FaultKind::IpcDisconnect),
            window,
        ),
        ops::FAULT_CLEAR => driver.clear_fault(
            FaultKind::parse(params["kind"].as_str().unwrap_or_default())
                .unwrap_or(FaultKind::IpcDisconnect),
            window,
        ),
        ops::WINDOW_REQUEST_CLOSE => driver.request_close(window),
        _ => unreachable!("step_op maps only closed-set ops"),
    };
    match result {
        Ok(reply) => StepOutcome::Ok(reply),
        Err(DriverError::Protocol(e)) if e.code == "unavailable" => StepOutcome::Unsupported(e),
        Err(e) => StepOutcome::Failed(e),
    }
}
