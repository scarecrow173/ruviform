//! `plugin-dock-lab` — the Issue #631 GPUI Test Lab / Scenario
//! Console GUI v1 (feature `lab-gui`).
//!
//! A real GPUI window that dogfoods the merged scenario catalog
//! through the shared `ruviform-ui` view vocabulary. The catalog
//! (`HEADLESS_MATRIX` headless cells plus `real_catalog` rows) is
//! projected once at startup into a `ViewSpec` tree
//! (`Panel`/`Label`/`Table`/`Button` — no GPUI widget code, no
//! second executor), and headless cells run through
//! `scenario::run_selected` exactly like `plugin-dock-console`.
//!
//! - The catalog table is a **capability projection** — `runnable`/
//!   `missing`/`session-required` declarations, never verdicts.
//! - `Run selected` executes the selected headless cell;
//!   `Run all headless` executes every headless cell. Real-window
//!   rows are listed but never executed here — they stay delegated
//!   to the opt-in `plugin-dock-real-scenario` lane
//!   (`mise run test-ui-plugin-dock-real`).
//! - Completed runs append `render_jsonl` blocks (`lane =
//!   "plugin-dock-lab"`) to `--out <path>` (or stdout), one block per
//!   run, so a session's evidence accumulates in order.
//! - A session that never reaches `WindowLifecycle::Ready` (headless
//!   or non-interactive host) fails closed: every catalog row is
//!   reported `NotRun` with `observed.environment_blocked` and the
//!   process exits `0` — never a fabricated pass.
//!
//! Exit code: `0` on a clean session end or the blocked path; `1` on
//! output-write failure or a driver-loop error after a live session;
//! `2` on argument misuse.
//!
//! **RT contract:** non-RT only — UI thread + scenario worker
//! threads, never an audio path. **Test-only:** nothing here ships.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use ruviform_audio_test::EnvironmentKind;
use ruviform_plugin_dock_test::LaneEnvironment;
use ruviform_plugin_dock_test::artifact::PLUGIN_PATH_ENV;
use ruviform_plugin_dock_test::jsonl::{render_jsonl, repo_sha};
use ruviform_plugin_dock_test::real_catalog;
use ruviform_plugin_dock_test::scenario::{HEADLESS_MATRIX, RUN_ID, run_selected};
use ruviform_ui::behavior::CommandIntent;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ButtonSpec, CellValue, ComponentKey, ComponentSpec,
    ComponentVisualState, LabelSpec, PanelSpec, TABLE_SELECT_ACTION, TableSelectionSpec, TableSpec,
    ViewHost, ViewSpec,
};
use ruviform_ui::{
    DriverEvent, Event, Intent, LogicalSize, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::{COMMAND_INTENT_OP, GpuiDriver, ViewMount};
use ruviform_ui_test::{
    CaseId, EntryProjection, RunId, ScenarioCapability, ScenarioDriver, ScenarioId, ScenarioReport,
    ScenarioVerdict,
};
use serde_json::{Value, json};

/// Run id stamped on lab-produced blocked reports — distinct from the
/// headless lane's `rf631-plugin-dock` so lab session evidence never
/// conflates with lane evidence (executed runs still carry the
/// catalog's own `RUN_ID`, same as `plugin-dock-console`).
const LAB_RUN_ID: &str = "rf631-test-lab";

/// Lane label written into every lab JSONL `run` header — the same
/// convention `plugin-dock-console` uses (`lane` names the consumer,
/// not the scenario).
const LAB_LANE: &str = "plugin-dock-lab";

/// Window title — also the root panel title.
const WINDOW_TITLE: &str = "Ruviform Test Lab — plugin-dock scenario console";

/// Upper bound for the interactive-session handshake — the same
/// value `RealOsGuiTest::COMMAND_TIMEOUT` uses. A window that has
/// not reported `Ready` within this bound is treated as
/// `environment_blocked`, never waited on forever.
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// Discrete action names the lab's buttons emit — matched in
/// [`plan_command`].
const ACTION_RUN_SELECTED: &str = "lab.run-selected";
const ACTION_RUN_ALL: &str = "lab.run-all";
const ACTION_QUIT: &str = "lab.quit";

const USAGE: &str = "usage: plugin-dock-lab [--out <path>]";

/// One projected catalog row — the GUI's only catalog shape. Built
/// once from `HEADLESS_MATRIX.project()` + `real_catalog`, never
/// mutated afterwards.
struct CatalogRow {
    /// Case id (also the `run_selected` predicate key for headless
    /// rows — case ids are unique across the matrix).
    case: &'static str,
    /// Scenario id.
    scenario: &'static str,
    /// Declared driver for this row.
    driver: ScenarioDriver,
    /// kebab-case driver name for display.
    driver_name: String,
    /// Declared required capabilities (kept for blocked reports).
    required_caps: &'static [ScenarioCapability],
    /// Comma-joined capability names for the table cell.
    required_text: String,
    /// Projection column text — `runnable` / `missing <cap>` /
    /// `session-required`. Never a verdict.
    projection: String,
    /// Lane column text — `headless` or `real_catalog::LANE`.
    lane: &'static str,
    /// `true` for real-window catalog rows — listed but never
    /// executed by the lab.
    real: bool,
}

/// A completed run's row in the results table.
#[derive(Clone)]
struct ResultRow {
    case: String,
    driver: String,
    verdict: String,
    detail: String,
}

/// Mutable lab state shared between the UI thread (event callback)
/// and read for spec rebuilds. Everything here is plain data — the
/// struct crosses into the app thread inside `Arc<Mutex<..>>`.
struct LabState {
    /// Status line text.
    status: String,
    /// Selected catalog row index (table `select` intents).
    selected: Option<usize>,
    /// Rows of the most recently completed run (empty until then).
    results: Vec<ResultRow>,
    /// Evidence note — JSONL destination + last write summary.
    evidence: String,
    /// Whether a scenario worker is currently running (run gate).
    running: bool,
}

/// What a recognized command intent should do — the pure decision
/// half of intent dispatch, kept separate so unit tests exercise it
/// without a window.
enum CommandPlan {
    /// Record a catalog row selection.
    Select(usize),
    /// Run one headless cell — `(case id, declared driver)` so the
    /// predicate stays exact even if a future matrix projects one
    /// case under several drivers.
    RunCase(&'static str, ScenarioDriver),
    /// Run every headless catalog cell.
    RunAll,
    /// Show a guidance/status message (receipt `applied`).
    Inform(String),
    /// Exit the event loop.
    Quit,
    /// Refuse the command (receipt `rejected` + status detail).
    Reject(String),
}

/// Everything the event callback needs, bundled so `on_event` stays
/// a thin match. Constructed inside the app thread — every field is
/// either `'static` data or a channel end owned by this session.
struct LabGui {
    /// Shared mutable state.
    state: Arc<Mutex<LabState>>,
    /// Immutable catalog rows.
    rows: Arc<Vec<CatalogRow>>,
    /// Lane environment for scenario workers.
    env: LaneEnvironment,
    /// JSONL destination — `None` writes to stdout.
    out: Option<PathBuf>,
    /// Repo SHA snapshot for JSONL headers.
    repo_sha: Option<String>,
    /// Result channel of the in-flight worker, if any.
    worker_rx: Option<Receiver<Vec<ScenarioReport>>>,
    /// The lab window id — `Destroyed` on it exits the loop.
    window: WindowId,
    /// `Ready` was already reported to the main thread.
    ready_sent: bool,
    /// Sender half of the main-thread readiness handshake.
    ready_tx: Sender<Result<(), String>>,
    /// Driver clone for `update_view` (the `&mut dyn WindowDriver`
    /// parameter cannot reach the inherent method).
    ui: GpuiDriver,
}

/// kebab-case name of a [`ScenarioCapability`] — the same spelling
/// `ScenarioCapability`'s serde produces.
fn capability_name(capability: ScenarioCapability) -> String {
    serde_json::to_value(capability)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// kebab-case name of a [`ScenarioDriver`].
fn driver_name(driver: ScenarioDriver) -> String {
    serde_json::to_value(driver)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Projects the merged catalog into GUI rows — headless cells first
/// (catalog order), then the real-window rows. This is the only
/// place catalog data enters the app; nothing is duplicated.
fn catalog_rows() -> Vec<CatalogRow> {
    let mut rows = Vec::new();
    for projection in HEADLESS_MATRIX.project() {
        rows.push(headless_row(&projection));
    }
    for row in real_catalog::REAL_CATALOG {
        rows.push(CatalogRow {
            case: row.case,
            scenario: row.scenario,
            driver: real_catalog::DRIVER,
            driver_name: driver_name(real_catalog::DRIVER),
            required_caps: row.required,
            required_text: row
                .required
                .iter()
                .map(|cap| capability_name(*cap))
                .collect::<Vec<_>>()
                .join(", "),
            projection: "session-required".to_string(),
            lane: real_catalog::LANE,
            real: true,
        });
    }
    rows
}

/// One headless `EntryProjection` → `CatalogRow`.
fn headless_row<C>(projection: &EntryProjection<'_, C>) -> CatalogRow {
    let projection_text = if projection.runnable {
        "runnable".to_string()
    } else {
        // `project()` guarantees `missing` is `Some` when not
        // runnable — the same capability `require_capabilities`
        // would name.
        format!(
            "missing {}",
            projection
                .missing
                .map_or_else(|| "unknown".to_string(), capability_name)
        )
    };
    CatalogRow {
        case: projection.entry.case,
        scenario: projection.entry.scenario,
        driver: projection.driver,
        driver_name: driver_name(projection.driver),
        required_caps: projection.entry.required,
        required_text: projection
            .entry
            .required
            .iter()
            .map(|cap| capability_name(*cap))
            .collect::<Vec<_>>()
            .join(", "),
        projection: projection_text,
        lane: "headless",
        real: false,
    }
}

fn label(key: &str, text: &str) -> ComponentSpec {
    ComponentSpec::Label(LabelSpec {
        key: ComponentKey::from(key),
        text: text.to_string(),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Text),
            label: Some(key.to_string()),
            ..A11ySpec::default()
        },
    })
}

fn button(key: &str, label_text: &str, action: &str) -> ComponentSpec {
    ComponentSpec::Button(ButtonSpec {
        key: ComponentKey::from(key),
        label: label_text.to_string(),
        target: key.to_string(),
        action: action.to_string(),
        params: None,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Button),
            label: Some(label_text.to_string()),
            ..A11ySpec::default()
        },
    })
}

fn table(
    key: &str,
    columns: &[&str],
    rows: Vec<Vec<Option<CellValue>>>,
    selectable: bool,
) -> ComponentSpec {
    ComponentSpec::Table(TableSpec {
        key: ComponentKey::from(key),
        columns: columns.iter().map(|c| (*c).to_string()).collect(),
        rows,
        selection: selectable.then(|| TableSelectionSpec {
            target: key.to_string(),
        }),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Table),
            label: Some(key.to_string()),
            ..A11ySpec::default()
        },
    })
}

fn panel(key: &str, title: Option<String>, children: Vec<ComponentSpec>) -> ComponentSpec {
    ComponentSpec::Panel(PanelSpec {
        key: ComponentKey::from(key),
        title,
        children,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Group),
            label: Some(key.to_string()),
            ..A11ySpec::default()
        },
    })
}

/// Builds the lab `ViewSpec` from current state — the whole tree is
/// rebuilt on each change and pushed through `update_view` (a dozen
/// rows makes diffing unnecessary).
fn lab_spec(state: &LabState, rows: &[CatalogRow]) -> ViewSpec {
    let catalog_rows: Vec<Vec<Option<CellValue>>> = rows
        .iter()
        .map(|row| {
            vec![
                Some(CellValue::Text(row.case.to_string())),
                Some(CellValue::Text(row.scenario.to_string())),
                Some(CellValue::Text(row.driver_name.clone())),
                Some(CellValue::Text(row.required_text.clone())),
                Some(CellValue::Text(row.projection.clone())),
                Some(CellValue::Text(row.lane.to_string())),
            ]
        })
        .collect();
    let result_rows: Vec<Vec<Option<CellValue>>> = state
        .results
        .iter()
        .map(|row| {
            vec![
                Some(CellValue::Text(row.case.clone())),
                Some(CellValue::Text(row.driver.clone())),
                Some(CellValue::Text(row.verdict.clone())),
                Some(CellValue::Text(row.detail.clone())),
            ]
        })
        .collect();
    ViewSpec {
        root: panel(
            "lab.root",
            Some(WINDOW_TITLE.to_string()),
            vec![
                label("lab.status", &state.status),
                label(
                    "lab.catalog.note",
                    "catalog — capability projection only (not execution evidence); \
                     real-window rows run via `mise run test-ui-plugin-dock-real`",
                ),
                table(
                    "lab.catalog",
                    &[
                        "case",
                        "scenario",
                        "driver",
                        "required",
                        "projection",
                        "lane",
                    ],
                    catalog_rows,
                    true,
                ),
                label("lab.results.note", "results — verdicts of executed runs"),
                table(
                    "lab.results",
                    &["case", "driver", "verdict", "detail"],
                    result_rows,
                    false,
                ),
                label("lab.evidence", &state.evidence),
                panel(
                    "lab.actions",
                    None,
                    vec![
                        button("lab.run-selected", "Run selected", ACTION_RUN_SELECTED),
                        button("lab.run-all", "Run all headless", ACTION_RUN_ALL),
                        button("lab.quit", "Quit", ACTION_QUIT),
                    ],
                ),
            ],
        ),
    }
}

/// Maps finished reports into results-table rows — verdict words
/// come from `ScenarioVerdict::as_str`, never re-spelled.
fn result_rows(reports: &[ScenarioReport]) -> Vec<ResultRow> {
    reports
        .iter()
        .map(|report| {
            let detail = report
                .detail
                .clone()
                .or_else(|| {
                    report
                        .observed
                        .as_ref()
                        .and_then(|observed| observed["observed"].as_str().map(str::to_owned))
                })
                .unwrap_or_else(|| report.verdict.as_str().to_string());
            ResultRow {
                case: report
                    .case
                    .as_ref()
                    .map_or_else(|| "-".to_string(), |case| case.as_str().to_string()),
                driver: driver_name(report.driver),
                verdict: report.verdict.as_str().to_string(),
                detail,
            }
        })
        .collect()
}

/// The pure half of command dispatch — decides what an `Action`
/// intent means without touching driver state. `running` gates
/// re-entrant run commands; `selected` is the catalog row index.
fn plan_command(
    action: &str,
    params: &Value,
    rows: &[CatalogRow],
    selected: Option<usize>,
    running: bool,
) -> CommandPlan {
    match action {
        TABLE_SELECT_ACTION => {
            let Some(row) = params.get("row").and_then(Value::as_u64) else {
                return CommandPlan::Reject("selection intent carried no row index".to_string());
            };
            let Ok(index) = usize::try_from(row) else {
                return CommandPlan::Reject(format!("catalog row {row} out of range"));
            };
            if index < rows.len() {
                CommandPlan::Select(index)
            } else {
                CommandPlan::Reject(format!("catalog row {index} out of range"))
            }
        }
        ACTION_RUN_SELECTED => {
            if running {
                return CommandPlan::Reject("a scenario run is already in progress".to_string());
            }
            let Some(index) = selected else {
                return CommandPlan::Reject(
                    "no catalog row selected — select a row first".to_string(),
                );
            };
            let Some(row) = rows.get(index) else {
                return CommandPlan::Reject(format!("catalog row {index} out of range"));
            };
            if row.real {
                CommandPlan::Inform(
                    "real-window cases are not executed by the lab — run the opt-in lane \
                     `mise run test-ui-plugin-dock-real`"
                        .to_string(),
                )
            } else {
                CommandPlan::RunCase(row.case, row.driver)
            }
        }
        ACTION_RUN_ALL => {
            if running {
                CommandPlan::Reject("a scenario run is already in progress".to_string())
            } else {
                CommandPlan::RunAll
            }
        }
        ACTION_QUIT => CommandPlan::Quit,
        other => CommandPlan::Reject(format!("unknown lab action `{other}`")),
    }
}

/// Spawns the scenario worker for `cell` (`None` = all headless
/// cells). The worker owns a cloned [`LaneEnvironment`] and sends
/// the finished reports back once; the UI thread drains the channel
/// on `Tick`. The run gate (`state.running`) is set here so a second
/// run command arriving before the first `Tick` still rejects.
fn spawn_run(lab: &mut LabGui, cell: Option<(&'static str, ScenarioDriver)>) {
    let env = lab.env.clone();
    let (tx, rx): (Sender<Vec<ScenarioReport>>, Receiver<Vec<ScenarioReport>>) = channel();
    lab.worker_rx = Some(rx);
    lock_state(&lab.state).running = true;
    thread::spawn(move || {
        let reports = run_selected(&env, |entry, driver| {
            cell.is_none_or(|(case, wanted)| entry.case == case && driver == wanted)
        });
        let _ = tx.send(reports);
    });
}

/// Locks shared state, tolerating a poisoned mutex (the lab keeps
/// running with last-known-good data rather than panicking the UI).
fn lock_state(state: &Mutex<LabState>) -> MutexGuard<'_, LabState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Applies a [`CommandPlan`] to lab state. Returns `true` when the
/// view should be repainted — `Quit` returns `false` because the
/// event loop is ending.
fn apply_plan(lab: &mut LabGui, plan: CommandPlan, drv: &mut dyn WindowDriver) -> bool {
    match plan {
        CommandPlan::Select(index) => {
            let mut state = lock_state(&lab.state);
            state.selected = Some(index);
            state.status = format!("selected `{}`", lab.rows[index].case);
        }
        CommandPlan::RunCase(case, driver) => {
            spawn_run(lab, Some((case, driver)));
            lock_state(&lab.state).status = format!("running `{case}`…");
        }
        CommandPlan::RunAll => {
            spawn_run(lab, None);
            lock_state(&lab.state).status = "running all headless cases…".to_string();
        }
        CommandPlan::Inform(message) => {
            lock_state(&lab.state).status = message;
        }
        CommandPlan::Quit => {
            drv.exit();
            return false;
        }
        CommandPlan::Reject(message) => {
            lock_state(&lab.state).status = format!("rejected: {message}");
        }
    }
    true
}

/// Handles one `rvf.ui.command.v1` intent: decode → plan → apply →
/// receipt. Foreign ops (non-command intents) are ignored — the
/// receipt contract only covers command intents.
fn handle_command_intent(
    lab: &mut LabGui,
    window: WindowId,
    intent: &Intent,
    drv: &mut dyn WindowDriver,
) {
    if intent.op != COMMAND_INTENT_OP {
        return;
    }
    let command = intent
        .params
        .get("intent")
        .cloned()
        .and_then(|value| serde_json::from_value::<CommandIntent>(value).ok());
    let plan = match command {
        Some(CommandIntent::Action { action, params, .. }) => {
            let params = params.unwrap_or(Value::Null);
            let (selected, running) = {
                let state = lock_state(&lab.state);
                (state.selected, state.running)
            };
            plan_command(&action, &params, lab.rows.as_slice(), selected, running)
        }
        _ => CommandPlan::Reject("unrecognized command intent".to_string()),
    };
    // Receipt status mirrors the plan: `Reject` is the only terminal
    // failure; everything else changed lab state (selection, run
    // dispatch, status text, exit request).
    let rejected = matches!(plan, CommandPlan::Reject(_));
    let repaint = apply_plan(lab, plan, drv);
    let status = if rejected { "rejected" } else { "applied" };
    drv.post_event(
        window,
        &Event::result_ok(intent.id.clone(), json!({ "status": status })),
    );
    if repaint {
        repaint_view(lab, window);
    }
}

/// Drains the worker channel on `Tick`: a finished run updates the
/// results table, appends its JSONL block, and repaints.
fn drain_worker(lab: &mut LabGui, window: WindowId) {
    let Some(rx) = lab.worker_rx.as_ref() else {
        return;
    };
    match rx.try_recv() {
        Ok(reports) => {
            lab.worker_rx = None;
            let rows = result_rows(&reports);
            let summary = format!(
                "{} case(s): {}",
                reports.len(),
                reports
                    .iter()
                    .map(|report| report.verdict.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            {
                let mut state = lock_state(&lab.state);
                state.results = rows;
                state.running = false;
                state.status = format!("run finished — {summary}");
            }
            match append_jsonl(lab, &reports) {
                Ok(note) => {
                    lock_state(&lab.state).evidence = note;
                }
                Err(message) => {
                    lock_state(&lab.state).status = format!("jsonl write failed: {message}");
                }
            }
            repaint_view(lab, window);
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {}
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            // Worker panicked without sending — honest failure
            // state, never a silent hang.
            lab.worker_rx = None;
            let mut state = lock_state(&lab.state);
            state.running = false;
            state.status = "run worker failed before reporting".to_string();
            drop(state);
            repaint_view(lab, window);
        }
    }
}

/// Appends one `render_jsonl` block for `reports` — `--out` file in
/// append mode (a session's runs accumulate) or stdout.
fn append_jsonl(lab: &LabGui, reports: &[ScenarioReport]) -> Result<String, String> {
    let text = render_jsonl(RUN_ID, LAB_LANE, lab.repo_sha.as_deref(), reports);
    if let Some(path) = &lab.out {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("cannot open `{}`: {error}", path.display()))?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
        Ok(format!(
            "appended {} case line(s) to `{}`",
            reports.len(),
            path.display()
        ))
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .and_then(|()| stdout.flush())
            .map_err(|error| format!("cannot write stdout: {error}"))?;
        Ok(format!("wrote {} case line(s) to stdout", reports.len()))
    }
}

/// Rebuilds the spec from current state and queues `update_view`.
fn repaint_view(lab: &LabGui, window: WindowId) {
    let spec = {
        let state = lock_state(&lab.state);
        lab_spec(&state, lab.rows.as_slice())
    };
    let _ = lab.ui.update_view(window, spec);
}

/// `NotRun` + `environment_blocked` reports for every catalog row —
/// the lab's blocked-session evidence. Mirrors
/// `real_dock::blocked_reports`: `environment_kind` stays
/// `deterministic-backend` because no real environment produced
/// these rows.
fn blocked_reports(env: &LaneEnvironment, rows: &[CatalogRow]) -> Vec<ScenarioReport> {
    let detail = "lab session could not start — no interactive window session";
    rows.iter()
        .map(|row| {
            let mut report = ScenarioReport::new(
                RunId::new(LAB_RUN_ID),
                ScenarioId::new(row.scenario),
                Some(CaseId::new(row.case)),
                row.driver,
            );
            report.required = row.required_caps.iter().copied().collect();
            report.verdict = ScenarioVerdict::NotRun;
            report.detail = Some(detail.to_string());
            report.observed = Some(json!({
                "observed": "environment_blocked",
                "detail": detail,
            }));
            report.environment = Some(json!({
                "os": env::consts::OS,
                "arch": env::consts::ARCH,
                "scale_factor": Value::Null,
                "repo_sha": env.repo_sha,
                "environment_kind": EnvironmentKind::DeterministicBackend.as_str(),
                "lane": LAB_LANE,
            }));
            report
        })
        .collect()
}

/// Writes the blocked-session JSONL and reports whether the write
/// succeeded (failure → exit 1, matching the lane binaries).
fn emit_blocked(env: &LaneEnvironment, rows: &[CatalogRow], out: Option<&PathBuf>) -> bool {
    let reports = blocked_reports(env, rows);
    let text = render_jsonl(LAB_RUN_ID, LAB_LANE, env.repo_sha.as_deref(), &reports);
    let result = if let Some(path) = out {
        // Append like `append_jsonl` — a blocked session must not
        // truncate evidence accumulated by earlier sessions/runs.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(text.as_bytes()))
            .map_err(|error| format!("{}: {error}", path.display()))
    } else {
        std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .and_then(|()| std::io::stdout().flush())
            .map_err(|error| error.to_string())
    };
    match result {
        Ok(()) => true,
        Err(message) => {
            eprintln!("plugin-dock-lab: cannot write output: {message}");
            false
        }
    }
}

/// Parsed command line — `--out <path>` only; everything else is
/// exit 2.
struct Options {
    /// JSONL destination; `None` writes to stdout.
    out: Option<PathBuf>,
}

/// `parse_args_from` outcome — `Help` prints usage and exits `0`,
/// kept distinct from `Options` so the pure parser stays testable.
enum ParseResult {
    Options(Options),
    Help,
}

/// Thin argument parse matching the lane binaries — `Err` carries
/// the message the caller prints before exiting `2`.
fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseResult, String> {
    let mut out = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a value".to_string());
                };
                out = Some(PathBuf::from(value));
            }
            "--help" | "-h" => return Ok(ParseResult::Help),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(ParseResult::Options(Options { out }))
}

/// Process-bound wrapper: maps `Err` to stderr + exit-2 signal and
/// `Help` to usage + exit-0.
fn parse_args() -> Option<Options> {
    match parse_args_from(env::args().skip(1)) {
        Ok(ParseResult::Options(options)) => Some(options),
        Ok(ParseResult::Help) => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Err(message) => {
            eprintln!("plugin-dock-lab: {message}\n{USAGE}");
            None
        }
    }
}

/// Spawns the GPUI app thread: constructs the `!Send` driver inside
/// the thread (the `RealOsGuiTest` precedent — only the plain-data
/// `ViewMount` crosses the boundary), creates the lab window, and
/// runs the event loop. `WindowLifecycle::Ready` on the lab window
/// is reported once through `ready_tx`; the returned handle yields
/// `Ok(Ok(()))` on clean exit, `Ok(Err)` on a driver/setup error,
/// `Err` on panic.
fn spawn_app(
    mount: ViewMount,
    state: Arc<Mutex<LabState>>,
    rows: Arc<Vec<CatalogRow>>,
    env: LaneEnvironment,
    repo_sha: Option<String>,
    out: Option<PathBuf>,
    ready_tx: Sender<Result<(), String>>,
) -> thread::JoinHandle<Result<(), String>> {
    thread::spawn(move || -> Result<(), String> {
        let mut driver = GpuiDriver::new();
        let window_spec = WindowSpec {
            title: WINDOW_TITLE.to_string(),
            logical_size: LogicalSize::new(1100, 860),
            position: None,
            resizable: true,
        };
        let window = driver
            .create_view_window(&window_spec, mount)
            .map_err(|error| error.to_string())?;
        let ui = driver.clone();
        let mut lab = LabGui {
            state,
            rows,
            env,
            out,
            repo_sha,
            worker_rx: None,
            window,
            ready_sent: false,
            ready_tx,
            ui,
        };
        driver
            .run(None, move |event, drv| match event {
                DriverEvent::Windowed {
                    window: event_window,
                    event: WindowLifecycle::Ready,
                } if event_window == lab.window => {
                    if !lab.ready_sent {
                        lab.ready_sent = true;
                        let _ = lab.ready_tx.send(Ok(()));
                    }
                }
                DriverEvent::Windowed {
                    window: event_window,
                    event: WindowLifecycle::Destroyed,
                } if event_window == lab.window => {
                    drv.exit();
                }
                DriverEvent::Windowed {
                    window: event_window,
                    event: WindowLifecycle::Action(intent),
                } => {
                    handle_command_intent(&mut lab, event_window, &intent, drv);
                }
                DriverEvent::Tick => {
                    let window = lab.window;
                    drain_worker(&mut lab, window);
                }
                _ => {}
            })
            .map_err(|error| error.to_string())
    })
}

/// Waits for the interactive-session handshake, then either joins
/// the app thread (live session — exit mirrors the loop result) or
/// emits the fail-closed blocked record (no session — exit 0 unless
/// the write itself fails).
fn finish_session(
    app: thread::JoinHandle<Result<(), String>>,
    ready_rx: &Receiver<Result<(), String>>,
    env: &LaneEnvironment,
    rows: &[CatalogRow],
    out: Option<&PathBuf>,
) -> ExitCode {
    match ready_rx.recv_timeout(READY_TIMEOUT) {
        Ok(Ok(())) => match app.join() {
            // Interactive session confirmed — the app thread ran
            // until the window closed or Quit; mirror its result.
            Ok(Ok(())) => ExitCode::SUCCESS,
            Ok(Err(message)) => {
                eprintln!("plugin-dock-lab: driver loop failed: {message}");
                ExitCode::FAILURE
            }
            Err(_) => {
                eprintln!("plugin-dock-lab: app thread panicked");
                ExitCode::FAILURE
            }
        },
        Ok(Err(message)) => {
            // Setup failed before the loop started — join (the
            // thread already returned) and record the blocked path.
            let _ = app.join();
            eprintln!("plugin-dock-lab: window setup failed: {message}");
            if emit_blocked(env, rows, out) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(_) => {
            // Timeout or disconnect: the session never proved
            // interactive. A still-running app thread is reaped by
            // process exit — the blocked record is written first.
            if emit_blocked(env, rows, out) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> ExitCode {
    let Some(options) = parse_args() else {
        return ExitCode::from(2);
    };
    let env = LaneEnvironment {
        repo_sha: repo_sha(),
        plugin_path: env::var_os(PLUGIN_PATH_ENV).map(PathBuf::from),
    };
    let rows = Arc::new(catalog_rows());
    let state = Arc::new(Mutex::new(LabState {
        status: "ready — select a catalog row, or run all headless cases".to_string(),
        selected: None,
        results: Vec::new(),
        evidence: match &options.out {
            Some(path) => format!("jsonl appends to `{}`", path.display()),
            None => "jsonl writes to stdout".to_string(),
        },
        running: false,
    }));

    // Mount on the main thread — `ViewHost`/`ViewMount` are plain
    // data and cross into the app thread; only the driver is `!Send`.
    let view = ViewInstanceId::new(631).expect("view instance id 631 is non-zero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    {
        let state = lock_state(&state);
        host.mount(lab_spec(&state, rows.as_slice()))
            .expect("lab view spec mounts");
    }
    host.drain_events();

    let (ready_tx, ready_rx) = channel::<Result<(), String>>();
    let app = spawn_app(
        ViewMount { host, theme: None },
        Arc::clone(&state),
        Arc::clone(&rows),
        env.clone(),
        env.repo_sha.clone(),
        options.out.clone(),
        ready_tx,
    );
    finish_session(app, &ready_rx, &env, rows.as_slice(), options.out.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rows() -> Vec<CatalogRow> {
        catalog_rows()
    }

    #[test]
    fn catalog_rows_cover_headless_and_real() {
        let rows = test_rows();
        let headless = HEADLESS_MATRIX.project().len();
        assert_eq!(rows.len(), headless + real_catalog::REAL_CATALOG.len());
        assert!(rows.iter().take(headless).all(|row| !row.real));
        assert!(rows.iter().skip(headless).all(|row| row.real));
        // Real rows are session-required and carry the real lane
        // label — never executable through the lab.
        for row in rows.iter().skip(headless) {
            assert_eq!(row.projection, "session-required");
            assert_eq!(row.lane, real_catalog::LANE);
            assert_eq!(row.driver, real_catalog::DRIVER);
        }
        // Headless rows carry the catalog's own driver/scenario.
        for projection in HEADLESS_MATRIX.project() {
            let row = rows
                .iter()
                .find(|row| row.case == projection.entry.case)
                .expect("every projected cell has a row");
            assert_eq!(row.scenario, projection.entry.scenario);
            assert_eq!(row.driver, projection.driver);
            assert_eq!(row.required_caps, projection.entry.required);
            assert!(!row.real);
        }
    }

    #[test]
    fn plan_command_dispatches_by_action() {
        let rows = test_rows();
        // Select: valid index plans Select; out-of-range rejects.
        assert!(matches!(
            plan_command(TABLE_SELECT_ACTION, &json!({"row": 0}), &rows, None, false),
            CommandPlan::Select(0)
        ));
        assert!(matches!(
            plan_command(
                TABLE_SELECT_ACTION,
                &json!({"row": rows.len() as u64}),
                &rows,
                None,
                false
            ),
            CommandPlan::Reject(_)
        ));
        // Run selected with no selection rejects.
        assert!(matches!(
            plan_command(ACTION_RUN_SELECTED, &Value::Null, &rows, None, false),
            CommandPlan::Reject(_)
        ));
        // A selected headless row runs its case.
        let headless_index = rows.iter().position(|row| !row.real).expect("headless row");
        let expected_case = rows[headless_index].case;
        assert!(matches!(
            plan_command(
                ACTION_RUN_SELECTED,
                &Value::Null,
                &rows,
                Some(headless_index),
                false
            ),
            CommandPlan::RunCase(case, driver)
                if case == expected_case && driver == rows[headless_index].driver
        ));
        // A selected real row informs instead of executing.
        let real_index = rows.iter().position(|row| row.real).expect("real row");
        assert!(matches!(
            plan_command(
                ACTION_RUN_SELECTED,
                &Value::Null,
                &rows,
                Some(real_index),
                false
            ),
            CommandPlan::Inform(_)
        ));
        // Run-all and quit.
        assert!(matches!(
            plan_command(ACTION_RUN_ALL, &Value::Null, &rows, None, false),
            CommandPlan::RunAll
        ));
        assert!(matches!(
            plan_command(ACTION_QUIT, &Value::Null, &rows, None, false),
            CommandPlan::Quit
        ));
        // Running gate + unknown action.
        assert!(matches!(
            plan_command(ACTION_RUN_ALL, &Value::Null, &rows, None, true),
            CommandPlan::Reject(_)
        ));
        assert!(matches!(
            plan_command("bogus.action", &Value::Null, &rows, None, false),
            CommandPlan::Reject(_)
        ));
    }

    #[test]
    fn blocked_reports_fail_closed_for_every_row() {
        let env = LaneEnvironment::default();
        let rows = test_rows();
        let reports = blocked_reports(&env, &rows);
        assert_eq!(reports.len(), rows.len());
        for (row, report) in rows.iter().zip(reports.iter()) {
            assert_eq!(report.case.as_ref().map(CaseId::as_str), Some(row.case));
            assert_eq!(report.scenario.as_str(), row.scenario);
            assert_eq!(report.driver, row.driver);
            assert_eq!(report.required, row.required_caps.iter().copied().collect());
            assert_eq!(report.verdict, ScenarioVerdict::NotRun);
            let observed = report.observed.as_ref().expect("observed recorded");
            assert_eq!(observed["observed"], "environment_blocked");
            let kind = report
                .environment
                .as_ref()
                .and_then(|env| env["environment_kind"].as_str());
            assert_eq!(kind, Some("deterministic-backend"));
            assert_ne!(kind, Some("real-window"), "nothing ran — no real claim");
        }
    }

    #[test]
    fn result_rows_use_verdict_vocabulary() {
        let env = LaneEnvironment::default();
        let rows = blocked_reports(&env, &test_rows());
        let mapped = result_rows(&rows);
        assert_eq!(mapped.len(), rows.len());
        for (report, row) in rows.iter().zip(mapped.iter()) {
            assert_eq!(row.verdict, report.verdict.as_str());
            assert_eq!(row.driver, driver_name(report.driver));
        }
    }

    #[test]
    fn lab_spec_builds_catalog_and_results_tables() {
        let rows = test_rows();
        let state = LabState {
            status: "test".to_string(),
            selected: Some(0),
            results: vec![ResultRow {
                case: "view-mount".to_string(),
                driver: "fake".to_string(),
                verdict: "pass".to_string(),
                detail: "ok".to_string(),
            }],
            evidence: "none".to_string(),
            running: false,
        };
        let spec = lab_spec(&state, &rows);
        // `ComponentSpec` is internally tagged (`{"type": ..., ...}`)
        // and `ComponentKey` is transparent — assert on the wire
        // shape directly.
        let json = serde_json::to_value(&spec).expect("spec serializes");
        let text = json.to_string();
        assert!(text.contains("lab.catalog"));
        assert!(text.contains("lab.results"));
        assert!(text.contains(ACTION_RUN_SELECTED));
        assert!(text.contains(ACTION_RUN_ALL));
        assert!(text.contains(ACTION_QUIT));
        let children = json["root"]["children"]
            .as_array()
            .expect("root panel children");
        let find_table = |key: &str| {
            children
                .iter()
                .find(|child| {
                    child["type"].as_str() == Some("table") && child["key"].as_str() == Some(key)
                })
                .unwrap_or_else(|| panic!("table `{key}` present"))
        };
        // The catalog table is selectable; the results table is not.
        assert!(find_table("lab.catalog")["selection"].is_object());
        assert!(find_table("lab.results")["selection"].is_null());
    }

    #[test]
    fn emit_blocked_appends_instead_of_truncating() {
        // A blocked session must accumulate evidence like a live one —
        // two blocked writes to the same `--out` keep both run blocks.
        let dir = env::temp_dir().join(format!("plugin-dock-lab-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("blocked.jsonl");
        let env = LaneEnvironment::default();
        let rows = test_rows();
        assert!(emit_blocked(&env, &rows, Some(&path)));
        assert!(emit_blocked(&env, &rows, Some(&path)));
        let text = std::fs::read_to_string(&path).expect("read back");
        let run_headers = text.matches("\"type\":\"run\"").count();
        assert_eq!(run_headers, 2, "both blocked blocks preserved");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_args_from_accepts_out_and_rejects_unknown() {
        fn args(items: &[&str]) -> std::vec::IntoIter<String> {
            items
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
                .into_iter()
        }
        match parse_args_from(args(&["--out", "evidence.jsonl"])).expect("parse") {
            ParseResult::Options(options) => {
                assert_eq!(
                    options.out.as_deref(),
                    Some(std::path::Path::new("evidence.jsonl"))
                );
            }
            ParseResult::Help => panic!("unexpected help"),
        }
        match parse_args_from(args(&[])).expect("parse") {
            ParseResult::Options(options) => assert!(options.out.is_none()),
            ParseResult::Help => panic!("unexpected help"),
        }
        assert!(parse_args_from(args(&["--bogus"])).is_err());
        assert!(parse_args_from(args(&["--out"])).is_err());
        assert!(matches!(
            parse_args_from(args(&["--help"])).expect("parse"),
            ParseResult::Help
        ));
    }
}
