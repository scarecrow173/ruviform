//! Issue #313 (RF-GT-04) — real-OS keyboard-input smoke, Windows slice.
//!
//! Manual lane (interactive Windows session — the window must be able
//! to take OS foreground focus):
//!
//! ```sh
//! mise run example-ui-gpui-real-os-smoke
//! # or directly:
//! cargo run -p ruviform-ui-gpui --features gpui --example real_os_smoke -- \
//!     --out evidence/rf313-os-capability-matrix/verdict-windows.json
//! ```
//!
//! What it does: mounts `Panel[Fader(gain), Seek(seek)]` — the two
//! value controls the Issue #479 keyboard convention covers — into a
//! real GPUI window, waits for `WindowLifecycle::Ready` and
//! `Focused(true)`, then injects `Tab, Right, Tab, Left` through Win32
//! `SendInput` (`ruviform_ui_gpui::os_input`) and verifies that the
//! rendered keyboard path produces `WindowLifecycle::Action` commit
//! intents on *both* controls with the expected value directions.
//!
//! Honesty contract (Issue #313): `SendInput` is OS automation
//! injection through the real foreground input path — it is *not*
//! physical manual input, and it is a different layer from the GPUI
//! test helpers (`dispatch_keystroke`/`simulate_keystrokes`), which
//! never leave the toolkit. Do not record this run as physical-input
//! evidence. Pointer input is out of scope for this slice.
//!
//! Exit codes: `0` = pass, `1` = fail, `2` = unsupported (non-Windows
//! hosts — `os_input` has no backend there).
//!
//! The example binds only the driver-neutral API plus the thin
//! `os_input` verification module — no `gpui` type appears here.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState,
    ControlSpec, Orientation, PanelSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{
    DriverEvent, Intent, LogicalSize, MonitorInfo, WindowDriver, WindowId, WindowLifecycle,
    WindowSpec,
};
use ruviform_ui_gpui::os_input::{self, OsKey};
use ruviform_ui_gpui::{COMMAND_INTENT_OP, GpuiDriver, ViewMount};
use serde_json::{Value, json};

/// Whole-run budget: Ready+focus, injection, and intent delivery must
/// all complete inside this window or the verdict is `fail`.
const TIMEOUT: Duration = Duration::from_secs(60);
/// Driver tick cadence — bounds event latency and the timeout check.
const TICK: Duration = Duration::from_millis(50);
/// Settle delay after `Focused(true)` before the first keystroke, so
/// the OS has actually made the window foreground.
const FOCUS_SETTLE: Duration = Duration::from_millis(400);
/// Gap between injected keystrokes — keeps each key a distinct OS
/// input event instead of a merged burst.
const KEY_GAP: Duration = Duration::from_millis(250);
/// Last-chance wait for the injector's completion report at verdict
/// time (the report and the final commit intent race across threads).
const REPORT_GRACE: Duration = Duration::from_millis(300);

/// Keystroke sequence: Tab onto the first value control, Right =
/// one positive effective step (#479), Tab onto the second control,
/// Left = one negative effective step.
const SMOKE_KEYS: [OsKey; 4] = [OsKey::Tab, OsKey::Right, OsKey::Tab, OsKey::Left];
/// The two `control_id`s the smoke spec mounts (`ControlSpec::key`
/// becomes the control id when `control_id` is `None`).
const VALUE_CONTROLS: [&str; 2] = ["gain", "seek"];

/// Injector → UI thread commands.
enum Cmd {
    /// Focus confirmed — run the keystroke sequence.
    Go,
    /// Verdict already reached — stop before injecting more keys.
    Abort,
}

/// One observed commit intent reduced to what the verdict checks.
struct CommitSeen {
    /// `params.intent.control_id`.
    control_id: String,
    /// `params.intent.value.normalized`.
    normalized: f64,
}

/// Mutable smoke state shared with the `run` event closure.
struct Smoke {
    /// The smoke window's id.
    window: WindowId,
    /// `WindowLifecycle::Ready` observed.
    ready: bool,
    /// `WindowLifecycle::Focused(true)` observed.
    focused: bool,
    /// `Go` already sent to the injector.
    injected: bool,
    /// `Focused(false)` events observed (informational).
    focus_lost: u32,
    /// Anomalies worth recording (focus request failure, …).
    notes: Vec<String>,
    /// Injector completion report, once received.
    inject_report: Option<Result<(), String>>,
    /// Every `WindowLifecycle::Action` intent observed, in order.
    intents: Vec<Intent>,
    /// Latest `WindowDriver::monitors()` snapshot.
    monitors: Vec<MonitorInfo>,
    /// Scale factors seen via events and monitors.
    scale_factors: Vec<f64>,
    /// Run start instant.
    started: Instant,
    /// Tick count (diagnostic).
    ticks: u64,
    /// UI → injector command channel.
    cmd_tx: mpsc::Sender<Cmd>,
    /// Injector → UI report channel.
    rep_rx: mpsc::Receiver<Result<(), String>>,
    /// Environment metadata collected before the window opened.
    meta: Value,
    /// Verdict output path (`--out` / `RF313_SMOKE_VERDICT`).
    out: Option<PathBuf>,
    /// Final verdict; `Some` also means the run is finished.
    result: Option<Result<(), String>>,
}

/// CLI options.
struct Opts {
    /// Verdict JSON destination; `None` prints to stdout.
    out: Option<PathBuf>,
    /// Revision override (`--revision` / `RF313_SMOKE_REVISION`).
    revision: Option<String>,
}

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn a11y_slider(label: &str, o: Orientation) -> A11ySpec {
    A11ySpec {
        role: Some(A11yRole::Slider),
        label: Some(label.to_string()),
        orientation: Some(o),
        value_min: Some(0.0),
        value_max: Some(1.0),
        value_text: None,
    }
}

fn control(name: &str, label: &str, o: Orientation) -> ControlSpec {
    ControlSpec {
        key: key(name),
        control_id: None,
        target: Some(format!("mixer.{name}")),
        descriptor: ValueDescriptor {
            default_value: Some(0.5),
            step: Some(0.05),
            ..ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: o,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: a11y_slider(label, o),
    }
}

/// The smoke view: exactly two tab stops so Tab traversal has a
/// deterministic pair of destinations.
fn smoke_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF313 OS smoke".to_string()),
            children: vec![
                ComponentSpec::Fader(control("gain", "Gain", Orientation::Vertical)),
                ComponentSpec::Seek(control("seek", "Seek", Orientation::Horizontal)),
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn parse_args() -> Opts {
    let mut out = None;
    let mut revision = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(v) = arg.strip_prefix("--out=") {
            out = Some(PathBuf::from(v));
        } else if arg == "--out" {
            out = args.next().map(PathBuf::from);
        } else if let Some(v) = arg.strip_prefix("--revision=") {
            revision = Some(v.to_string());
        } else if arg == "--revision" {
            revision = args.next();
        } else if arg == "--help" || arg == "-h" {
            println!(
                "real_os_smoke — Issue #313 real-OS keyboard smoke (Windows)\n\
                 usage: real_os_smoke [--out <verdict.json>] [--revision <rev>]\n\
                 env:   RF313_SMOKE_VERDICT, RF313_SMOKE_REVISION"
            );
            std::process::exit(0);
        } else {
            eprintln!("[real_os_smoke] warning: ignoring argument {arg}");
        }
    }
    if out.is_none()
        && let Ok(v) = std::env::var("RF313_SMOKE_VERDICT")
        && !v.is_empty()
    {
        out = Some(PathBuf::from(v));
    }
    Opts { out, revision }
}

/// OS version + GPU name via CIM, best-effort. Any failure degrades
/// to `"unavailable"` — metadata honesty, never a fabricated value.
#[cfg(windows)]
fn host_info() -> Value {
    const SCRIPT: &str = "$os = Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty Caption; $gpu = Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name; [pscustomobject]@{ os_version = $os; gpu = $gpu } | ConvertTo-Json -Compress";
    let parsed = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            SCRIPT,
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok());
    parsed.unwrap_or_else(|| json!({"os_version": "unavailable", "gpu": "unavailable"}))
}

#[cfg(not(windows))]
fn host_info() -> Value {
    json!({"os_version": "unavailable", "gpu": "unavailable"})
}

/// `git rev-parse HEAD` inside the crate dir (works from a worktree —
/// `.git` is a file there, and git resolves it upward).
fn git_revision() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let rev = String::from_utf8(out.stdout).ok()?;
    let rev = rev.trim();
    if rev.is_empty() {
        None
    } else {
        Some(rev.to_string())
    }
}

fn resolve_revision(opt: Option<&str>) -> String {
    if let Some(r) = opt
        && !r.is_empty()
    {
        return r.to_string();
    }
    if let Ok(r) = std::env::var("RF313_SMOKE_REVISION")
        && !r.is_empty()
    {
        return r;
    }
    git_revision().unwrap_or_else(|| "unknown".to_string())
}

/// Current time as `(unix_ms, utc_iso8601)`.
fn now_utc() -> (u64, String) {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    (ms, unix_ms_to_utc(ms))
}

/// Days-from-civil inverse (Howard Hinnant's algorithm) — UTC ISO-8601
/// without pulling in a datetime dependency.
fn unix_ms_to_utc(ms: u64) -> String {
    let secs = ms / 1000;
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let sod = secs % 86_400;
    let z = days.saturating_add(719_468);
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let hh = sod / 3600;
    let mm = (sod % 3600) / 60;
    let ss = sod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Environment metadata collected once before the window opens.
fn collect_meta(opts: &Opts) -> Value {
    let host = host_info();
    let (unix_ms, utc) = now_utc();
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "os_version": host["os_version"].clone(),
        "gpu": host["gpu"].clone(),
        "revision": resolve_revision(opts.revision.as_deref()),
        "exe": std::env::current_exe()
            .map_or_else(|_| "unknown".to_string(), |p| p.display().to_string()),
        "timestamp_unix_ms": unix_ms,
        "timestamp_utc": utc,
    })
}

/// Injector thread body: wait for `Go`, settle, then press the smoke
/// keys with gaps. Any `SendInput` failure is reported and the
/// sequence stops — a partial run never reports success.
fn injector(rx: &mpsc::Receiver<Cmd>, tx: &mpsc::Sender<Result<(), String>>) {
    if !matches!(rx.recv(), Ok(Cmd::Go)) {
        return;
    }
    std::thread::sleep(FOCUS_SETTLE);
    for (i, key) in SMOKE_KEYS.into_iter().enumerate() {
        if i > 0 {
            std::thread::sleep(KEY_GAP);
        }
        // Abort wins over the next keystroke — never inject into a
        // window the verdict already abandoned.
        if rx.try_recv().is_ok() {
            return;
        }
        if let Err(e) = os_input::press_key(key) {
            let _ = tx.send(Err(format!("{}: {e}", key.name())));
            return;
        }
    }
    let _ = tx.send(Ok(()));
}

/// Reduces one `Action` intent's params to the fields the verdict
/// records — `null` for absent fields, never fabricated.
fn intent_summary(intent: &Intent) -> Value {
    let inner = &intent.params["intent"];
    json!({
        "id": intent.id,
        "op": intent.op,
        "kind": inner["kind"],
        "control_id": inner["control_id"],
        "target": inner["target"],
        "atomic": inner["atomic"],
        "value": inner["value"],
    })
}

/// Commit intents on the smoke controls, in arrival order. Only
/// `kind == "commit"` + `atomic == true` count — the #479 keyboard
/// path synthesizes an atomic begin/commit pair per key press.
fn value_commits(intents: &[Intent]) -> Vec<CommitSeen> {
    intents
        .iter()
        .filter_map(|intent| {
            if intent.op != COMMAND_INTENT_OP {
                return None;
            }
            let inner = &intent.params["intent"];
            if inner["kind"].as_str() != Some("commit") || inner["atomic"].as_bool() != Some(true) {
                return None;
            }
            let control_id = inner["control_id"].as_str()?.to_string();
            if !VALUE_CONTROLS.contains(&control_id.as_str()) {
                return None;
            }
            let normalized = inner["value"]["normalized"].as_f64()?;
            Some(CommitSeen {
                control_id,
                normalized,
            })
        })
        .collect()
}

/// The pass pattern: a commit above the 0.5 default (the Right
/// step) and a commit below it (the Left step) on *different*
/// controls — proving Tab moved focus between the two tab stops.
fn expected_commits_seen(commits: &[CommitSeen]) -> bool {
    commits.iter().any(|u| {
        u.normalized > 0.5
            && commits
                .iter()
                .any(|d| d.normalized < 0.5 && d.control_id != u.control_id)
    })
}

fn monitors_json(monitors: &[MonitorInfo]) -> Value {
    Value::Array(
        monitors
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "width": m.width,
                    "height": m.height,
                    "x": m.x,
                    "y": m.y,
                    "scale_factor": m.scale_factor,
                })
            })
            .collect(),
    )
}

/// Builds the verdict document. `reasons` is empty iff the run
/// passed; `notes` carries anomalies that did not decide the verdict.
fn verdict_doc(st: &Smoke, verdict: &str, reasons: &[String], commits: &[CommitSeen]) -> Value {
    let intents: Vec<Value> = st.intents.iter().map(intent_summary).collect();
    let commit_json: Vec<Value> = commits
        .iter()
        .map(|c| json!({"control_id": c.control_id, "normalized": c.normalized}))
        .collect();
    let checks = json!([
        {"name": "ready", "ok": st.ready},
        {"name": "os_focused", "ok": st.focused},
        {
            "name": "inject_ok",
            "ok": matches!(st.inject_report, Some(Ok(())))
        },
        {
            "name": "commits_on_both_controls_with_direction",
            "ok": expected_commits_seen(commits)
        },
    ]);
    json!({
        "schema": "rf313-real-os-smoke/v1",
        "issue": 313,
        "lane": "windows-sendinput-keyboard",
        "input_method": "win32-SendInput",
        "input_method_note": "OS automation injection through the real foreground input path — not physical manual input; distinct from GPUI simulated input (dispatch_keystroke/simulate_keystrokes), which never leaves the toolkit",
        "verdict": verdict,
        "reasons": reasons,
        "notes": st.notes,
        "environment": st.meta,
        "run": {
            "window": st.window.to_string(),
            "ready": st.ready,
            "focused": st.focused,
            "focus_lost_count": st.focus_lost,
            "scale_factors": st.scale_factors,
            "monitors": monitors_json(&st.monitors),
            "keystrokes": SMOKE_KEYS.into_iter().map(OsKey::name).collect::<Vec<_>>(),
            "inject_result": match &st.inject_report {
                Some(Ok(())) => json!("ok"),
                Some(Err(e)) => json!(e),
                None => Value::Null,
            },
            "value_commits": commit_json,
            "intents": intents,
            "checks": checks,
            "ticks": st.ticks,
            "elapsed_ms": u64::try_from(st.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    })
}

/// Writes the verdict JSON to `--out`/`RF313_SMOKE_VERDICT` or stdout.
fn write_verdict(doc: &Value, out: Option<&Path>) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(doc).unwrap_or_else(|_| doc.to_string());
    match out {
        Some(path) => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, format!("{text}\n"))?;
            eprintln!("[real_os_smoke] verdict written to {}", path.display());
        }
        None => println!("{text}"),
    }
    Ok(())
}

fn drain_reports(st: &mut Smoke) {
    while let Ok(report) = st.rep_rx.try_recv() {
        st.inject_report = Some(report);
    }
}

/// Sends `Go` once both `Ready` and `Focused(true)` have been
/// observed — never inject into a window whose OS focus is unproven.
fn maybe_inject(st: &mut Smoke) {
    if st.ready && st.focused && !st.injected {
        st.injected = true;
        if st.cmd_tx.send(Cmd::Go).is_err() {
            st.notes
                .push("injector channel closed before Go".to_string());
        }
    }
}

/// Reaches a verdict and shuts the loop down. `early` carries a
/// decisive failure (timeout, destroyed window); all other reasons
/// are derived from observed state so the verdict stays honest.
fn finish(st: &mut Smoke, drv: &mut dyn WindowDriver, early: Option<String>) {
    if st.result.is_some() {
        return;
    }
    drain_reports(st);
    // The injector's completion report and the final commit intent
    // race across threads — give the report a brief grace window.
    if st.inject_report.is_none()
        && let Ok(report) = st.rep_rx.recv_timeout(REPORT_GRACE)
    {
        st.inject_report = Some(report);
    }
    st.monitors = drv.monitors();
    for m in &st.monitors {
        if !st.scale_factors.contains(&m.scale_factor) {
            st.scale_factors.push(m.scale_factor);
        }
    }

    let commits = value_commits(&st.intents);
    let mut reasons: Vec<String> = Vec::new();
    if let Some(e) = early {
        reasons.push(e);
    }
    if !st.ready {
        reasons.push("WindowLifecycle::Ready not observed".to_string());
    }
    if !st.focused {
        reasons.push("WindowLifecycle::Focused(true) not observed".to_string());
    }
    match &st.inject_report {
        Some(Ok(())) => {}
        Some(Err(e)) => reasons.push(format!("key injection failed: {e}")),
        None => reasons.push("key injection did not run to completion".to_string()),
    }
    if !expected_commits_seen(&commits) {
        reasons.push(
            "expected commit intents (atomic, kind=commit) with a +step value \
             above the 0.5 default and a -step value below it on two distinct \
             controls were not both observed"
                .to_string(),
        );
    }

    let verdict = if reasons.is_empty() { "pass" } else { "fail" };
    let doc = verdict_doc(st, verdict, &reasons, &commits);
    let mut result = if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons.join("; "))
    };
    if let Err(e) = write_verdict(&doc, st.out.as_deref()) {
        eprintln!("[real_os_smoke] failed to write verdict file: {e}");
        eprintln!("{doc}");
        result = Err(format!("verdict write failed: {e}"));
    }
    eprintln!("[real_os_smoke] verdict: {verdict}");
    st.result = Some(result);
    // Stop the injector before it can press keys into whatever window
    // has focus after we leave.
    let _ = st.cmd_tx.send(Cmd::Abort);
    drv.exit();
}

/// Pass check after each event — finishes as soon as the expected
/// commit pair has been observed.
fn try_finish_if_passed(st: &mut Smoke, drv: &mut dyn WindowDriver) {
    if st.result.is_some() {
        return;
    }
    let commits = value_commits(&st.intents);
    if expected_commits_seen(&commits) {
        finish(st, drv, None);
    }
}

fn on_window(st: &mut Smoke, event: WindowLifecycle, drv: &mut dyn WindowDriver) {
    match event {
        WindowLifecycle::Ready => {
            st.ready = true;
            st.monitors = drv.monitors();
            if let Err(e) = drv.focus(st.window, true) {
                st.notes.push(format!("focus request failed: {e}"));
            }
            maybe_inject(st);
        }
        WindowLifecycle::Focused(true) => {
            st.focused = true;
            maybe_inject(st);
        }
        WindowLifecycle::Focused(false) => {
            st.focus_lost += 1;
        }
        WindowLifecycle::ScaleFactorChanged { factor } => {
            st.scale_factors.push(factor);
        }
        WindowLifecycle::Action(intent) => {
            eprintln!("[intent] op={} params={}", intent.op, intent.params);
            st.intents.push(intent);
        }
        WindowLifecycle::Destroyed => {
            finish(st, drv, Some("window destroyed before verdict".to_string()));
        }
        _ => {}
    }
    try_finish_if_passed(st, drv);
}

fn on_tick(st: &mut Smoke, drv: &mut dyn WindowDriver) {
    st.ticks += 1;
    drain_reports(st);
    if st.started.elapsed() > TIMEOUT {
        finish(
            st,
            drv,
            Some(format!("timed out after {}s", TIMEOUT.as_secs())),
        );
        return;
    }
    try_finish_if_passed(st, drv);
}

fn run_smoke(opts: Opts, meta: Value) -> ExitCode {
    let mut driver = GpuiDriver::new();
    // One `AttachEpochAllocator` per session — same rule as
    // `component_showcase` (never mint a second one for a fresh host).
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero view instance"),
        epochs.mint(),
    );
    host.mount(smoke_spec()).expect("mount smoke view");
    let window = driver
        .create_view_window(
            &WindowSpec {
                title: "RF313 real OS smoke".to_string(),
                logical_size: LogicalSize::new(480, 320),
                position: None,
                resizable: true,
            },
            ViewMount { host, theme: None },
        )
        .expect("create_view_window");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let (rep_tx, rep_rx) = mpsc::channel::<Result<(), String>>();
    // The `move` closure owns the channels for the thread's lifetime;
    // `injector` itself only needs shared references (`recv`/`send`
    // take `&self`).
    std::thread::spawn(move || injector(&cmd_rx, &rep_tx));

    let state = Rc::new(RefCell::new(Smoke {
        window,
        ready: false,
        focused: false,
        injected: false,
        focus_lost: 0,
        notes: Vec::new(),
        inject_report: None,
        intents: Vec::new(),
        monitors: Vec::new(),
        scale_factors: Vec::new(),
        started: Instant::now(),
        ticks: 0,
        cmd_tx,
        rep_rx,
        meta,
        out: opts.out,
        result: None,
    }));
    let handler = state.clone();
    driver
        .run(Some(TICK), move |event, drv| {
            let mut st = handler.borrow_mut();
            match event {
                DriverEvent::Tick => on_tick(&mut st, drv),
                DriverEvent::Windowed { window: w, event } if w == window => {
                    on_window(&mut st, event, drv);
                }
                _ => {}
            }
        })
        .expect("gpui driver run");

    let st = state.borrow();
    match &st.result {
        Some(Ok(())) => ExitCode::SUCCESS,
        Some(Err(_)) => ExitCode::FAILURE,
        None => {
            eprintln!("[real_os_smoke] run returned without a verdict");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let opts = parse_args();
    let meta = collect_meta(&opts);
    if !os_input::supported() {
        // Honest unsupported: no fake injection, no pass. Exit 2 is
        // distinct from fail (1) so automation can tell "never ran"
        // apart from "ran and broke".
        let reasons = vec![format!(
            "os_input: real OS keyboard injection is implemented for Windows only (os={})",
            std::env::consts::OS
        )];
        let doc = unsupported_doc(&meta, &reasons);
        if let Err(e) = write_verdict(&doc, opts.out.as_deref()) {
            eprintln!("[real_os_smoke] failed to write verdict file: {e}");
            eprintln!("{doc}");
        }
        eprintln!("[real_os_smoke] verdict: unsupported");
        return ExitCode::from(2);
    }
    run_smoke(opts, meta)
}

/// Minimal verdict document for platforms without an `os_input`
/// backend — same schema, `run` fields null.
fn unsupported_doc(meta: &Value, reasons: &[String]) -> Value {
    json!({
        "schema": "rf313-real-os-smoke/v1",
        "issue": 313,
        "lane": "windows-sendinput-keyboard",
        "input_method": "win32-SendInput",
        "input_method_note": "OS automation injection through the real foreground input path — not physical manual input; distinct from GPUI simulated input (dispatch_keystroke/simulate_keystrokes), which never leaves the toolkit",
        "verdict": "unsupported",
        "reasons": reasons,
        "notes": [],
        "environment": meta,
        "run": Value::Null,
    })
}
