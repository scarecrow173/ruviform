//! Issue #313 (RF-GT-04) — real-OS pointer-input smoke, Windows slice.
//!
//! Manual lane (interactive Windows session — the window must be
//! visible on the desktop and the real cursor is moved):
//!
//! ```sh
//! mise run example-ui-gpui-real-os-pointer-smoke
//! # or directly:
//! cargo run -p ruviform-ui-gpui --features gpui --example real_os_pointer_smoke -- \
//!     --out evidence/rf313-os-capability-matrix/verdict-pointer-windows.json
//! ```
//!
//! What it does: mounts `Panel[Fader(gain), Seek(seek)]` into a real
//! GPUI window, waits for `WindowLifecycle::Ready` and
//! `Focused(true)`, then aims OS-level pointer input through measured
//! geometry — `GpuiDriver::component_bounds` (last-painted logical
//! bounds) + `GpuiDriver::window_geometry` (client-area screen rect,
//! physical px) — and injects `move/down/move/up` plus a click via
//! Win32 `SendInput` (`ruviform_ui_gpui::os_input`). It verifies that
//! the real OS pointer path produces gesture (`atomic != true`)
//! `Begin`/`Preview`/`Commit` intents on *both* controls with the
//! expected normalized values.
//!
//! Honesty contract (Issue #313): `SendInput` is OS automation
//! injection through the real OS input path — it is *not* physical
//! manual input, and it is a different layer from the GPUI test
//! helpers (`dispatch_input`/`simulate_mouse_*`), which never leave
//! the toolkit. Do not record this run as physical-input evidence.
//! The smoke moves the user's real cursor and restores its position
//! on exit — run it on an interactive desktop, not over unsaved
//! work.
//!
//! Exit codes: `0` = pass, `1` = fail, `2` = unsupported (non-Windows
//! hosts — `os_input` has no backend there).
//!
//! The example binds only the driver-neutral API plus the thin
//! `os_input`/`OsWindowGeometry` verification surface — no `gpui`
//! type appears here.

use std::cell::RefCell;
use std::collections::BTreeMap;
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
    DriverEvent, Intent, LogicalRect, LogicalSize, MonitorInfo, WindowDriver, WindowId,
    WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::os_input;
use ruviform_ui_gpui::{COMMAND_INTENT_OP, GpuiDriver, OsWindowGeometry, ViewMount};
use serde_json::{Value, json};

/// Whole-run budget: Ready+focus, geometry queries, injection, and
/// intent delivery must all complete inside this window or the
/// verdict is `fail`.
const TIMEOUT: Duration = Duration::from_secs(60);
/// Driver tick cadence — bounds event latency and the timeout check.
const TICK: Duration = Duration::from_millis(50);
/// Settle delay after `Focused(true)` before the first pointer move,
/// so the OS has actually made the window foreground.
const FOCUS_SETTLE: Duration = Duration::from_millis(400);
/// Gap between injected pointer actions — keeps each OS input event
/// distinct instead of a merged burst.
const ACT_GAP: Duration = Duration::from_millis(200);
/// Delay before each cursor read-back so `SendInput` has landed.
const READBACK_SETTLE: Duration = Duration::from_millis(80);
/// Cursor read-back tolerance in physical px — beyond this the
/// commanded point and the observed cursor disagree.
const CURSOR_TOLERANCE: i32 = 4;
/// Normalized-value tolerance for intent assertions — covers
/// physical-px rounding and sub-pixel layout, not semantic drift.
const VALUE_TOLERANCE: f64 = 0.1;

/// Fraction pairs `(fx, fy)` inside a component's logical bounds.
/// `gain` is a vertical Fader: top = max, so `y_frac 0.75` maps to
/// normalized `0.25` and `y_frac 0.25` maps to `0.75` — the drag
/// runs bottom→top (value rises). `seek` is horizontal: `x_frac`
/// maps directly, so `0.8` clicks near the right end.
const GAIN_DRAG_FROM: (f64, f64) = (0.5, 0.75);
const GAIN_DRAG_TO: (f64, f64) = (0.5, 0.25);
const SEEK_CLICK: (f64, f64) = (0.8, 0.5);
/// Expected normalized values at the commanded fractions.
const GAIN_BEGIN_N: f64 = 0.25;
const GAIN_COMMIT_N: f64 = 0.75;
const SEEK_N: f64 = 0.8;

/// Injector → UI thread commands.
enum Cmd {
    /// Focus confirmed and targets computed — run the pointer
    /// sequence. Carries the physical-px screen points.
    Go {
        gain_from: (i32, i32),
        gain_to: (i32, i32),
        seek: (i32, i32),
    },
    /// Verdict already reached — stop before injecting more input.
    Abort,
}

/// One cursor read-back recorded by the injector.
#[derive(Debug, Clone, Copy)]
struct CursorRead {
    /// Commanded physical-px target.
    commanded: (i32, i32),
    /// `GetCursorPos` result after the move settled.
    actual: (i32, i32),
}

/// Injector completion report.
#[derive(Debug)]
struct InjectReport {
    /// Cursor read-backs in command order.
    reads: Vec<CursorRead>,
}

/// One observed command intent reduced to what the verdict checks.
#[derive(Debug)]
struct IntentSeen {
    /// `params.intent.kind` (`begin`/`preview`/`commit`/…).
    kind: String,
    /// `params.intent.control_id`.
    control_id: String,
    /// `params.intent.gesture_id` — pointer gestures share one id
    /// across begin/preview/commit; keyboard atomic pairs differ.
    gesture_id: Option<String>,
    /// `params.intent.atomic` — `true` only for the keyboard
    /// convention's synthesized pairs; pointer gestures are not
    /// atomic.
    atomic: Option<bool>,
    /// `params.intent.value.normalized`.
    normalized: Option<f64>,
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
    inject_report: Option<Result<InjectReport, String>>,
    /// Every `WindowLifecycle::Action` intent observed, in order.
    intents: Vec<Intent>,
    /// Latest `WindowDriver::monitors()` snapshot.
    monitors: Vec<MonitorInfo>,
    /// Scale factors seen via events and monitors.
    scale_factors: Vec<f64>,
    /// `window_geometry` result, once obtained.
    geometry: Option<OsWindowGeometry>,
    /// `component_bounds` snapshot, once both controls appear.
    bounds: Option<BTreeMap<ComponentKey, LogicalRect>>,
    /// Run start instant.
    started: Instant,
    /// Tick count (diagnostic).
    ticks: u64,
    /// UI → injector command channel.
    cmd_tx: mpsc::Sender<Cmd>,
    /// Injector → UI report channel.
    rep_rx: mpsc::Receiver<Result<InjectReport, String>>,
    /// Environment metadata collected before the window opened.
    meta: Value,
    /// Verdict output path (`--out` / `RF313_POINTER_SMOKE_VERDICT`).
    out: Option<PathBuf>,
    /// Cursor position saved before the run — restored on exit.
    cursor_home: Option<(i32, i32)>,
    /// Final verdict; `Some` also means the run is finished.
    result: Option<Result<(), String>>,
}

/// CLI options.
struct Opts {
    /// Verdict JSON destination; `None` prints to stdout.
    out: Option<PathBuf>,
    /// Revision override (`--revision` / `RF313_POINTER_SMOKE_REVISION`).
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

/// The smoke view: a vertical Fader (drag target) and a horizontal
/// Seek (click target).
fn smoke_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF313 OS pointer smoke".to_string()),
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
                "real_os_pointer_smoke — Issue #313 real-OS pointer smoke (Windows)\n\
                 usage: real_os_pointer_smoke [--out <verdict.json>] [--revision <rev>]\n\
                 env:   RF313_POINTER_SMOKE_VERDICT, RF313_POINTER_SMOKE_REVISION"
            );
            std::process::exit(0);
        } else {
            eprintln!("[real_os_pointer_smoke] warning: ignoring argument {arg}");
        }
    }
    if out.is_none()
        && let Ok(v) = std::env::var("RF313_POINTER_SMOKE_VERDICT")
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
    if let Ok(r) = std::env::var("RF313_POINTER_SMOKE_REVISION")
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

/// A `(fx, fy)` fraction inside `rect` → physical screen point:
/// `client_origin + (rect.origin + fraction * rect.size) * scale`.
/// Returns `None` when `rect` is degenerate — a zero-size component
/// has no clickable interior and must not be aimed at its edge.
#[expect(
    clippy::cast_possible_truncation,
    reason = "screen coordinates are bounded by the OS virtual screen (thousands of px), far inside i32"
)]
fn screen_point(
    geom: &OsWindowGeometry,
    rect: &LogicalRect,
    frac: (f64, f64),
) -> Option<(i32, i32)> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let scale = geom.scale_factor;
    let lx = rect.x + frac.0 * rect.width;
    let ly = rect.y + frac.1 * rect.height;
    let px = f64::from(geom.origin_x) + lx * scale;
    let py = f64::from(geom.origin_y) + ly * scale;
    Some((px.round() as i32, py.round() as i32))
}

/// Moves the cursor to `target`, settles, then reads back
/// `GetCursorPos` — evidence that the OS cursor actually reached the
/// commanded point, not just that `SendInput` accepted the event.
fn move_and_read(target: (i32, i32), reads: &mut Vec<CursorRead>) -> Result<(), String> {
    os_input::move_to(target.0, target.1).map_err(|e| format!("move_to: {e}"))?;
    std::thread::sleep(READBACK_SETTLE);
    let actual = os_input::cursor_position().map_err(|e| format!("cursor_position: {e}"))?;
    reads.push(CursorRead {
        commanded: target,
        actual,
    });
    Ok(())
}

/// Injector thread body: wait for `Go`, settle, then run the pointer
/// sequence — drag `gain` bottom→top, click `seek` at 80%. Any
/// `SendInput` failure is reported and the sequence stops — a
/// partial run never reports success.
fn injector(rx: &mpsc::Receiver<Cmd>, tx: &mpsc::Sender<Result<InjectReport, String>>) {
    let Ok(Cmd::Go {
        gain_from,
        gain_to,
        seek,
    }) = rx.recv()
    else {
        return;
    };
    std::thread::sleep(FOCUS_SETTLE);
    let mut reads = Vec::new();
    let result = (|| -> Result<(), String> {
        // Drag on the vertical Fader: hover → press → drag up →
        // release. Produces Begin/Preview/Commit on `gain`.
        move_and_read(gain_from, &mut reads)?;
        if rx.try_recv().is_ok() {
            return Ok(());
        }
        os_input::mouse_down().map_err(|e| format!("mouse_down: {e}"))?;
        std::thread::sleep(ACT_GAP);
        move_and_read(gain_to, &mut reads)?;
        std::thread::sleep(ACT_GAP);
        os_input::mouse_up().map_err(|e| format!("mouse_up: {e}"))?;
        std::thread::sleep(ACT_GAP);
        // Click on the horizontal Seek: move → press → release.
        // Produces Begin/Commit on `seek`.
        move_and_read(seek, &mut reads)?;
        if rx.try_recv().is_ok() {
            return Ok(());
        }
        os_input::mouse_down().map_err(|e| format!("mouse_down(seek): {e}"))?;
        std::thread::sleep(ACT_GAP);
        os_input::mouse_up().map_err(|e| format!("mouse_up(seek): {e}"))?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            let _ = tx.send(Ok(InjectReport { reads }));
        }
        Err(e) => {
            let _ = tx.send(Err(e));
        }
    }
}

/// Reduces one `Action` intent's params to the fields the verdict
/// records — `None` for absent fields, never fabricated.
fn intent_summary(intent: &Intent) -> Value {
    let inner = &intent.params["intent"];
    json!({
        "id": intent.id,
        "op": intent.op,
        "kind": inner["kind"],
        "control_id": inner["control_id"],
        "target": inner["target"],
        "gesture_id": inner["gesture_id"],
        "atomic": inner["atomic"],
        "value": inner["value"],
    })
}

/// All `rvf.ui.command.v1` intents reduced for verdict checks.
fn command_intents(intents: &[Intent]) -> Vec<IntentSeen> {
    intents
        .iter()
        .filter(|i| i.op == COMMAND_INTENT_OP)
        .filter_map(|intent| {
            let inner = &intent.params["intent"];
            Some(IntentSeen {
                kind: inner["kind"].as_str()?.to_string(),
                control_id: inner["control_id"].as_str()?.to_string(),
                gesture_id: inner["gesture_id"].as_str().map(str::to_string),
                atomic: inner["atomic"].as_bool(),
                normalized: inner["value"]["normalized"].as_f64(),
            })
        })
        .collect()
}

/// Intents on `control` of `kind` whose normalized value is within
/// `tolerance` of `expected`.
fn matching<'a>(
    seen: &'a [IntentSeen],
    control: &str,
    kind: &str,
    expected: f64,
) -> Vec<&'a IntentSeen> {
    seen.iter()
        .filter(|i| {
            i.control_id == control
                && i.kind == kind
                && i.normalized
                    .is_some_and(|n| (n - expected).abs() <= VALUE_TOLERANCE)
        })
        .collect()
}

/// The gain drag verdict: a non-atomic `begin` near `GAIN_BEGIN_N`,
/// a `commit` near `GAIN_COMMIT_N`, both sharing one `gesture_id`.
fn gain_drag_ok(seen: &[IntentSeen]) -> bool {
    let begins = matching(seen, "gain", "begin", GAIN_BEGIN_N);
    let commits = matching(seen, "gain", "commit", GAIN_COMMIT_N);
    begins.iter().any(|b| {
        b.atomic != Some(true)
            && commits.iter().any(|c| {
                c.atomic != Some(true) && c.gesture_id.is_some() && c.gesture_id == b.gesture_id
            })
    })
}

/// The seek click verdict: non-atomic `begin` + `commit` near
/// `SEEK_N` sharing one `gesture_id`.
fn seek_click_ok(seen: &[IntentSeen]) -> bool {
    let begins = matching(seen, "seek", "begin", SEEK_N);
    let commits = matching(seen, "seek", "commit", SEEK_N);
    begins.iter().any(|b| {
        b.atomic != Some(true)
            && commits.iter().any(|c| {
                c.atomic != Some(true) && c.gesture_id.is_some() && c.gesture_id == b.gesture_id
            })
    })
}

/// Count of `preview` intents on `gain` — informational only:
/// previews are contract-coalescible and may legitimately drop to
/// zero, so they never gate the verdict.
fn gain_previews(seen: &[IntentSeen]) -> usize {
    seen.iter()
        .filter(|i| i.control_id == "gain" && i.kind == "preview")
        .count()
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

fn geometry_json(geom: Option<&OsWindowGeometry>) -> Value {
    match geom {
        Some(g) => json!({
            "origin_x": g.origin_x,
            "origin_y": g.origin_y,
            "width": g.width,
            "height": g.height,
            "scale_factor": g.scale_factor,
        }),
        None => Value::Null,
    }
}

fn bounds_json(bounds: Option<&BTreeMap<ComponentKey, LogicalRect>>) -> Value {
    match bounds {
        Some(map) => Value::Object(
            map.iter()
                .map(|(k, r)| {
                    (
                        k.0.clone(),
                        json!({
                            "x": r.x,
                            "y": r.y,
                            "width": r.width,
                            "height": r.height,
                        }),
                    )
                })
                .collect(),
        ),
        None => Value::Null,
    }
}

/// Builds the verdict document. `reasons` is empty iff the run
/// passed; `notes` carries anomalies that did not decide the verdict.
fn verdict_doc(st: &Smoke, verdict: &str, reasons: &[String], seen: &[IntentSeen]) -> Value {
    let intents: Vec<Value> = st.intents.iter().map(intent_summary).collect();
    let reads_json: Vec<Value> = st
        .inject_report
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|r| {
            r.reads
                .iter()
                .map(|c| {
                    json!({
                        "commanded": [c.commanded.0, c.commanded.1],
                        "actual": [c.actual.0, c.actual.1],
                        "delta_px": [
                            (c.actual.0 - c.commanded.0).abs(),
                            (c.actual.1 - c.commanded.1).abs(),
                        ],
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let cursor_ok = st
        .inject_report
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .is_some_and(|r| {
            r.reads.iter().all(|c| {
                (c.actual.0 - c.commanded.0).abs() <= CURSOR_TOLERANCE
                    && (c.actual.1 - c.commanded.1).abs() <= CURSOR_TOLERANCE
            })
        });
    let checks = json!([
        {"name": "ready", "ok": st.ready},
        {"name": "os_focused", "ok": st.focused},
        {
            "name": "geometry_and_bounds",
            "ok": st.geometry.is_some()
                && st
                    .bounds
                    .as_ref()
                    .is_some_and(|b| b.contains_key(&key("gain")) && b.contains_key(&key("seek")))
        },
        {
            "name": "inject_ok",
            "ok": matches!(st.inject_report, Some(Ok(_)))
        },
        {"name": "cursor_reached_targets", "ok": cursor_ok},
        {"name": "gain_drag_begin_preview_commit", "ok": gain_drag_ok(seen)},
        {"name": "seek_click_begin_commit", "ok": seek_click_ok(seen)},
    ]);
    json!({
        "schema": "rf313-real-os-pointer-smoke/v1",
        "issue": 313,
        "lane": "windows-sendinput-pointer",
        "input_method": "win32-SendInput-mouse-absolute",
        "input_method_note": "OS automation injection through the real OS input path — not physical manual input; distinct from GPUI simulated input (dispatch_input/simulate_mouse_*), which never leaves the toolkit. The real cursor is moved and restored on exit.",
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
            "geometry": geometry_json(st.geometry.as_ref()),
            "component_bounds": bounds_json(st.bounds.as_ref()),
            "targets": {
                "gain_drag": {"from_frac": GAIN_DRAG_FROM, "to_frac": GAIN_DRAG_TO,
                              "expect_begin_n": GAIN_BEGIN_N, "expect_commit_n": GAIN_COMMIT_N},
                "seek_click": {"frac": SEEK_CLICK, "expect_n": SEEK_N},
                "value_tolerance": VALUE_TOLERANCE,
                "cursor_tolerance_px": CURSOR_TOLERANCE,
            },
            "inject_result": match &st.inject_report {
                Some(Ok(_)) => json!("ok"),
                Some(Err(e)) => json!(e),
                None => Value::Null,
            },
            "cursor_reads": reads_json,
            "gain_preview_count": gain_previews(seen),
            "command_intents": seen.iter().map(|i| json!({
                "kind": i.kind,
                "control_id": i.control_id,
                "gesture_id": i.gesture_id,
                "atomic": i.atomic,
                "normalized": i.normalized,
            })).collect::<Vec<_>>(),
            "intents": intents,
            "checks": checks,
            "ticks": st.ticks,
            "elapsed_ms": u64::try_from(st.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    })
}

/// Writes the verdict JSON to `--out`/`RF313_POINTER_SMOKE_VERDICT` or stdout.
fn write_verdict(doc: &Value, out: Option<&Path>) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(doc).unwrap_or_else(|_| doc.to_string());
    match out {
        Some(path) => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, format!("{text}\n"))?;
            eprintln!(
                "[real_os_pointer_smoke] verdict written to {}",
                path.display()
            );
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

/// Queries `window_geometry` + `component_bounds` until both carry
/// the two controls — bounds only exist after the first paint, so
/// this retries on each tick rather than assuming Ready implies
/// layout.
fn try_collect_geometry(st: &mut Smoke, query: &GpuiDriver) {
    if st.geometry.is_none() {
        match query.window_geometry(st.window) {
            Ok(g) => st.geometry = Some(g),
            Err(e) => st.notes.push(format!("window_geometry not ready: {e}")),
        }
    }
    if st.bounds.is_none() {
        match query.component_bounds(st.window) {
            Ok(map) if map.contains_key(&key("gain")) && map.contains_key(&key("seek")) => {
                st.bounds = Some(map);
            }
            Ok(_) => {}
            Err(e) => st.notes.push(format!("component_bounds not ready: {e}")),
        }
    }
}

/// Computes the three physical target points once geometry and
/// bounds are both available. `None` while either is missing or a
/// control rect is degenerate.
fn compute_targets(st: &Smoke) -> Option<Cmd> {
    let geom = st.geometry.as_ref()?;
    let bounds = st.bounds.as_ref()?;
    let gain = bounds.get(&key("gain"))?;
    let seek = bounds.get(&key("seek"))?;
    Some(Cmd::Go {
        gain_from: screen_point(geom, gain, GAIN_DRAG_FROM)?,
        gain_to: screen_point(geom, gain, GAIN_DRAG_TO)?,
        seek: screen_point(geom, seek, SEEK_CLICK)?,
    })
}

/// Sends `Go` once Ready + Focused(true) + geometry/bounds have all
/// been observed — never inject into a window whose state is
/// unproven.
fn maybe_inject(st: &mut Smoke, query: &GpuiDriver) {
    if st.ready && st.focused && !st.injected {
        try_collect_geometry(st, query);
        if let Some(cmd) = compute_targets(st) {
            st.injected = true;
            if st.cmd_tx.send(cmd).is_err() {
                st.notes
                    .push("injector channel closed before Go".to_string());
            }
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
        && let Ok(report) = st.rep_rx.recv_timeout(Duration::from_millis(300))
    {
        st.inject_report = Some(report);
    }
    st.monitors = drv.monitors();
    for m in &st.monitors {
        if !st.scale_factors.contains(&m.scale_factor) {
            st.scale_factors.push(m.scale_factor);
        }
    }

    let seen = command_intents(&st.intents);
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
    if st.geometry.is_none() {
        reasons.push("window_geometry query never succeeded".to_string());
    }
    let bounds_ready = st
        .bounds
        .as_ref()
        .is_some_and(|b| b.contains_key(&key("gain")) && b.contains_key(&key("seek")));
    if !bounds_ready {
        reasons.push("component_bounds never reported gain+seek".to_string());
    }
    match &st.inject_report {
        Some(Ok(_)) => {}
        Some(Err(e)) => reasons.push(format!("pointer injection failed: {e}")),
        None => reasons.push("pointer injection did not run to completion".to_string()),
    }
    if !st
        .inject_report
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .is_some_and(|r| {
            r.reads.iter().all(|c| {
                (c.actual.0 - c.commanded.0).abs() <= CURSOR_TOLERANCE
                    && (c.actual.1 - c.commanded.1).abs() <= CURSOR_TOLERANCE
            })
        })
    {
        reasons.push("cursor read-back deviated from commanded targets".to_string());
    }
    if !gain_drag_ok(&seen) {
        reasons.push(
            "expected non-atomic gain gesture (begin ≈0.25 → commit ≈0.75, \
             same gesture_id) was not observed"
                .to_string(),
        );
    }
    if !seek_click_ok(&seen) {
        reasons.push(
            "expected non-atomic seek gesture (begin+commit ≈0.8, same gesture_id) \
             was not observed"
                .to_string(),
        );
    }

    let verdict = if reasons.is_empty() { "pass" } else { "fail" };
    let doc = verdict_doc(st, verdict, &reasons, &seen);
    let mut result = if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons.join("; "))
    };
    if let Err(e) = write_verdict(&doc, st.out.as_deref()) {
        eprintln!("[real_os_pointer_smoke] failed to write verdict file: {e}");
        eprintln!("{doc}");
        result = Err(format!("verdict write failed: {e}"));
    }
    eprintln!("[real_os_pointer_smoke] verdict: {verdict}");
    st.result = Some(result);
    // Restore the user's cursor before leaving — best effort, the
    // verdict is already decided.
    if let Some(home) = st.cursor_home
        && let Err(e) = os_input::move_to(home.0, home.1)
    {
        eprintln!("[real_os_pointer_smoke] cursor restore failed: {e}");
    }
    // Stop the injector before it can inject into whatever window
    // has focus after we leave.
    let _ = st.cmd_tx.send(Cmd::Abort);
    drv.exit();
}

/// Pass check after each event — finishes as soon as both expected
/// gestures have been observed.
fn try_finish_if_passed(st: &mut Smoke, drv: &mut dyn WindowDriver) {
    if st.result.is_some() {
        return;
    }
    let seen = command_intents(&st.intents);
    if gain_drag_ok(&seen) && seek_click_ok(&seen) {
        finish(st, drv, None);
    }
}

fn on_window(
    st: &mut Smoke,
    event: WindowLifecycle,
    drv: &mut dyn WindowDriver,
    query: &GpuiDriver,
) {
    match event {
        WindowLifecycle::Ready => {
            st.ready = true;
            st.monitors = drv.monitors();
            if let Err(e) = drv.focus(st.window, true) {
                st.notes.push(format!("focus request failed: {e}"));
            }
            maybe_inject(st, query);
        }
        WindowLifecycle::Focused(true) => {
            st.focused = true;
            maybe_inject(st, query);
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

fn on_tick(st: &mut Smoke, drv: &mut dyn WindowDriver, query: &GpuiDriver) {
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
    // Bounds may land a frame or two after Ready — keep polling
    // until the injector has gone or the verdict is reached.
    if !st.injected {
        try_collect_geometry(st, query);
        maybe_inject(st, query);
    }
    try_finish_if_passed(st, drv);
}

fn run_smoke(opts: Opts, meta: Value, cursor_home: Option<(i32, i32)>) -> ExitCode {
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
                title: "RF313 real OS pointer smoke".to_string(),
                logical_size: LogicalSize::new(480, 320),
                position: None,
                resizable: true,
            },
            ViewMount { host, theme: None },
        )
        .expect("create_view_window");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let (rep_tx, rep_rx) = mpsc::channel::<Result<InjectReport, String>>();
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
        geometry: None,
        bounds: None,
        started: Instant::now(),
        ticks: 0,
        cmd_tx,
        rep_rx,
        meta,
        out: opts.out,
        cursor_home,
        result: None,
    }));
    let handler = state.clone();
    let run_window = window;
    // `component_bounds`/`window_geometry` are inherent `GpuiDriver`
    // methods, unreachable through the `&mut dyn WindowDriver` the
    // callback receives — a clone shares the same `Rc<RefCell>`
    // state and is safe to query while the handler runs (the driver
    // takes the handler out before invoking it, so `inner` is
    // unborrowed at that point).
    let query = driver.clone();
    driver
        .run(Some(TICK), move |event, drv| {
            let mut st = handler.borrow_mut();
            match event {
                DriverEvent::Tick => on_tick(&mut st, drv, &query),
                DriverEvent::Windowed { window: w, event } if w == run_window => {
                    on_window(&mut st, event, drv, &query);
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
            eprintln!("[real_os_pointer_smoke] run returned without a verdict");
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
            "os_input: real OS pointer injection is implemented for Windows only (os={})",
            std::env::consts::OS
        )];
        let doc = unsupported_doc(&meta, &reasons);
        if let Err(e) = write_verdict(&doc, opts.out.as_deref()) {
            eprintln!("[real_os_pointer_smoke] failed to write verdict file: {e}");
            eprintln!("{doc}");
        }
        eprintln!("[real_os_pointer_smoke] verdict: unsupported");
        return ExitCode::from(2);
    }
    // Save the user's cursor position up front — restored on exit.
    let cursor_home = os_input::cursor_position()
        .map_err(|e| eprintln!("[real_os_pointer_smoke] cursor_position probe failed: {e}"))
        .ok();
    run_smoke(opts, meta, cursor_home)
}

/// Minimal verdict document for platforms without an `os_input`
/// backend — same schema, `run` fields null.
fn unsupported_doc(meta: &Value, reasons: &[String]) -> Value {
    json!({
        "schema": "rf313-real-os-pointer-smoke/v1",
        "issue": 313,
        "lane": "windows-sendinput-pointer",
        "input_method": "win32-SendInput-mouse-absolute",
        "input_method_note": "OS automation injection through the real OS input path — not physical manual input; distinct from GPUI simulated input (dispatch_input/simulate_mouse_*), which never leaves the toolkit. The real cursor is moved and restored on exit.",
        "verdict": "unsupported",
        "reasons": reasons,
        "notes": [],
        "environment": meta,
        "run": Value::Null,
    })
}
