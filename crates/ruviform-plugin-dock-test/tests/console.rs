//! Integration coverage for `plugin-dock-console` — the headless
//! scenario console over the merged catalog (Issue #631 console
//! slice).
//!
//! Asserted contracts:
//!
//! - `--list` emits capability-projection rows only — never verdict
//!   vocabulary, never an execution claim;
//! - `--run` selectors filter `HEADLESS_MATRIX.cells()` (OR within a
//!   flag, AND across flags), execute through the same catalog path
//!   as the lane, and render the same JSONL shape under lane
//!   `plugin-dock-console`;
//! - every refusal path is fail-closed exit 2 — unknown selectors,
//!   empty intersections, CLI misuse, real-lane run attempts, and
//!   `--include-real` without the `real-dock` feature.

use std::process::{Command, Output};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_plugin-dock-console");

/// The nine headless case ids in catalog order.
const HEADLESS_CASES: [&str; 9] = [
    "view-mount",
    "command-receipt",
    "artifact",
    "host-lifecycle",
    "editor-attach",
    "session-lifecycle",
    "host-terminate",
    "open-or-degrade",
    "dock-pixels",
];

/// Verdict vocabulary that must never appear in `--list` output —
/// a listing is a capability projection, not evidence.
const VERDICT_WORDS: [&str; 5] = ["pass", "fail", "not_run", "unsupported", "blocked"];

fn console(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("plugin-dock-console must spawn")
}

fn stdout_lines(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("non-JSON line `{line}`: {error}"))
        })
        .collect()
}

/// `(case, verdict)` pairs of the `case` rows in JSONL output.
fn case_rows(output: &Output) -> Vec<(String, String)> {
    stdout_lines(output)
        .into_iter()
        .filter(|row| row["type"] == "case")
        .map(|row| {
            (
                row["case"].as_str().unwrap_or_default().to_string(),
                row["verdict"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// `--list` emits exactly the nine headless catalog cells as
/// `type:"catalog"` rows carrying `runnable`/`missing` — and no
/// verdict key or verdict vocabulary anywhere.
#[test]
fn list_emits_headless_projection_without_verdicts() {
    let output = console(&["--list"]);
    assert!(
        output.status.success(),
        "--list must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = stdout_lines(&output);
    assert_eq!(rows.len(), 9, "nine headless catalog rows");
    for row in &rows {
        assert_eq!(row["type"], "catalog");
        assert!(row["case"].as_str().is_some());
        assert!(row["scenario"].as_str().is_some());
        assert!(row["driver"].as_str().is_some());
        assert!(row["required"].is_array());
        assert!(row.get("runnable").is_some(), "runnable present");
        assert!(row.get("missing").is_some(), "missing present");
        assert!(row.get("verdict").is_none(), "no verdict key: {row}");
    }
    let cases: Vec<&str> = rows.iter().filter_map(|row| row["case"].as_str()).collect();
    assert_eq!(cases, HEADLESS_CASES, "catalog order preserved");
    let text = String::from_utf8_lossy(&output.stdout);
    for word in VERDICT_WORDS {
        assert!(
            !text.contains(&format!("\"{word}\"")),
            "verdict vocabulary `{word}` leaked into --list output"
        );
    }
}

/// `--list --include-real` appends the twenty-seven real-lane rows marked
/// `session_required`/`lane` — metadata only, `runnable`/`missing`
/// null, no execution claim. Requires the `real-dock` feature; the
/// refusal path is covered by `include_real_refused_without_feature`.
#[cfg(feature = "real-dock")]
#[test]
fn list_include_real_appends_real_lane_rows() {
    let output = console(&["--list", "--include-real"]);
    assert!(
        output.status.success(),
        "--include-real must succeed under real-dock: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = stdout_lines(&output);
    assert_eq!(rows.len(), 36, "9 headless + 27 real rows");
    let real: Vec<&Value> = rows.iter().skip(9).collect();
    let expected = [
        "real-window-mount",
        "real-native-slot",
        "real-editor-attach",
        "real-capture",
        "real-dcomp-pixel",
        "real-dcomp-foreground",
        "real-dcomp-present",
        "real-dcomp-uncovered",
        "real-dcomp-bandescape",
        "real-dcomp-childinv",
        "real-multiwindow-focus",
        "real-os-input",
        "real-command-receipt",
        "real-window-dpi",
        "real-window-ime",
        "real-window-close",
        "real-editor-resize",
        "real-editor-redock",
        "real-session-terminate",
        "real-session-degraded",
        "real-session-recovery",
        "real-session-timeout",
        "real-editor-visibility",
        "real-editor-input",
        "real-editor-focus",
        "real-session-crash",
        "real-session-hang",
    ];
    for (row, case) in real.iter().zip(expected.iter()) {
        assert_eq!(row["type"], "catalog");
        assert_eq!(row["case"].as_str(), Some(*case));
        assert_eq!(row["scenario"].as_str(), Some("rf631.plugin-dock-real"));
        assert_eq!(row["driver"].as_str(), Some("real-os"));
        assert_eq!(row["session_required"], true);
        assert_eq!(row["lane"].as_str(), Some("real-window-dock"));
        assert!(row["runnable"].is_null(), "real rows claim no runnability");
        assert!(row["missing"].is_null());
        assert!(row.get("verdict").is_none());
    }
}

/// Without `real-dock` the real lane is not compiled in — the
/// console refuses `--include-real` honestly (exit 2) instead of
/// silently omitting the requested rows.
#[cfg(not(feature = "real-dock"))]
#[test]
fn include_real_refused_without_feature() {
    let output = console(&["--list", "--include-real"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "--include-real without real-dock must exit 2"
    );
    assert!(output.stdout.is_empty(), "no listing emitted on refusal");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("real-dock"), "stderr: {stderr}");
}

/// Bare `--run` executes all nine headless cells through the same
/// catalog path as the lane — JSONL header carries run id
/// `rf631-plugin-dock` and lane `plugin-dock-console`, and every
/// verdict is an honest recorded outcome (never fabricated).
#[test]
fn run_executes_full_catalog() {
    let output = console(&["--run"]);
    assert!(
        output.status.success(),
        "--run must not fail on honest non-pass verdicts: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = stdout_lines(&output);
    assert_eq!(rows[0]["type"], "run", "JSONL run header first");
    assert_eq!(rows[0]["run_id"], "rf631-plugin-dock");
    assert_eq!(rows[0]["lane"], "plugin-dock-console");
    let cases = case_rows(&output);
    assert_eq!(cases.len(), 9, "one case row per catalog cell");
    assert_eq!(
        cases
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        HEADLESS_CASES,
        "catalog order preserved"
    );
    for (_, verdict) in &cases {
        assert!(
            VERDICT_WORDS.contains(&verdict.as_str()),
            "recorded verdict must be vocabulary: {verdict}"
        );
    }
}

/// `--run --case view-mount` selects exactly one cell — the fake
/// driver mount case, which passes without any artifact.
#[test]
fn run_single_case() {
    let output = console(&["--run", "--case", "view-mount"]);
    assert!(output.status.success());
    let rows = case_rows(&output);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "view-mount");
    assert_eq!(rows[0].1, "pass");
}

/// Repeated `--case` flags are OR-ed within the dimension.
#[test]
fn run_case_selector_or() {
    let output = console(&["--run", "--case", "view-mount", "--case", "artifact"]);
    assert!(output.status.success());
    let rows = case_rows(&output);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "view-mount");
    assert_eq!(rows[1].0, "artifact");
}

/// Selector dimensions AND together: `--case artifact --driver
/// fake` matches no headless cell (artifact binds
/// `plugin-host-probe`) and must fail closed with exit 2 — nothing
/// executes, nothing is emitted.
#[test]
fn run_selector_dimensions_and_empty_is_refused() {
    let output = console(&["--run", "--case", "artifact", "--driver", "fake"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "no JSONL on refused selector");
}

/// `--driver fake` selects the two fake-bound cells;
/// `--scenario rf631.plugin-dock` selects all nine.
#[test]
fn run_driver_and_scenario_selectors() {
    let output = console(&["--run", "--driver", "fake"]);
    assert!(output.status.success());
    let rows = case_rows(&output);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "view-mount");
    assert_eq!(rows[1].0, "command-receipt");

    let output = console(&["--run", "--scenario", "rf631.plugin-dock"]);
    assert!(output.status.success());
    assert_eq!(case_rows(&output).len(), 9);
}

/// `editor-attach` alone reports `NotRun` — the `artifact`
/// prerequisite genuinely did not run in this invocation, so the
/// case fails closed rather than fabricating a pass. `NotRun` is a
/// recorded outcome: exit 0.
#[test]
fn run_editor_attach_alone_is_not_run() {
    let output = console(&["--run", "--case", "editor-attach"]);
    assert!(
        output.status.success(),
        "NotRun is a recorded outcome, not a lane failure"
    );
    let rows = case_rows(&output);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "not_run");
}

/// `artifact` + `editor-attach` selected together execute in catalog
/// order: artifact first, then editor-attach reports its honest
/// verdict — `unsupported` when the fixture resolved (no editor),
/// `not_run` when the artifact itself was unavailable. Never `pass`.
#[test]
fn run_artifact_then_editor_attach() {
    let output = console(&["--run", "--case", "artifact", "--case", "editor-attach"]);
    assert!(output.status.success());
    let rows = case_rows(&output);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "artifact");
    assert_eq!(rows[1].0, "editor-attach");
    assert!(
        rows[1].1 == "unsupported" || rows[1].1 == "not_run",
        "editor-attach must stay honest: {:?}",
        rows[1].1
    );
}

/// Unknown selector values are usage errors — exit 2, no execution.
#[test]
fn unknown_selectors_refused() {
    for args in [
        vec!["--run", "--case", "no-such-case"],
        vec!["--run", "--scenario", "no.such.scenario"],
        vec!["--run", "--driver", "no-such-driver"],
    ] {
        let output = console(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?} must exit 2: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{args:?} emitted output");
    }
}

/// Real-lane selectors are refused with guidance to the opt-in
/// real lane — the console never builds a second real executor.
#[test]
fn real_lane_selectors_refused() {
    for args in [
        vec!["--run", "--case", "real-window-mount"],
        vec!["--run", "--scenario", "rf631.plugin-dock-real"],
        vec!["--run", "--driver", "real-os"],
    ] {
        let output = console(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?} must exit 2: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{args:?} emitted output");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("plugin-dock-real-scenario"),
            "refusal must name the real lane: {stderr}"
        );
    }
}

/// CLI misuse — no mode, both modes, misplaced flags, unknown
/// arguments — is always exit 2.
#[test]
fn cli_misuse_refused() {
    for args in [
        vec![],
        vec!["--list", "--run"],
        vec!["--include-real", "--run"],
        vec!["--list", "--case", "view-mount"],
        vec!["--bogus"],
        vec!["--run", "--case"],
    ] {
        let output = console(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?} must exit 2: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// `--out <path>` writes the same JSONL to a file instead of stdout.
#[test]
fn run_out_writes_file() {
    let path = std::env::temp_dir().join(format!(
        "plugin-dock-console-test-{}-{}.jsonl",
        std::process::id(),
        "out"
    ));
    let path_arg = path.to_string_lossy().to_string();
    let output = console(&["--run", "--case", "view-mount", "--out", &path_arg]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "--out suppresses stdout");
    let written = std::fs::read_to_string(&path).expect("--out file written");
    let _ = std::fs::remove_file(&path);
    let rows: Vec<Value> = written
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL line"))
        .collect();
    assert_eq!(rows[0]["type"], "run");
    assert_eq!(rows[0]["lane"], "plugin-dock-console");
    assert_eq!(rows.len(), 2, "header + one case row");
}

/// `--help` exits 0 with usage on stdout.
#[test]
fn help_exits_zero() {
    let output = console(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("plugin-dock-console"));
}
