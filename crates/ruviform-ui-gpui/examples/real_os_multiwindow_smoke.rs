//! Real-OS multiwindow smoke — Issue #243 Windows slice.
//!
//! Drives two (later three) real GPUI windows with mounted views and
//! verifies the Window/Dock contract through **real OS input**
//! (`SendInput` keyboard/pointer injection via `os_input`, plus an
//! example-local chord injector for modifier chords `os_input` does
//! not cover). This is OS automation injection — not physical manual
//! input, and not GPUI simulated dispatch.
//!
//! Covered on a real Windows desktop:
//! - per-window focus transitions driven by real clicks;
//! - `focus(_, false)` release → `Unsupported`, unknown-window focus
//!   → `Host` (API honesty checks);
//! - keyboard value commits (Tab focus traversal + End/Left/Right)
//!   producing `rvf.ui.command.v1` intents;
//! - view state surviving `update_view` spec replacement, and NOT
//!   surviving window destruction (fresh host on regenerate);
//! - `DockWorkspace` detach/reattach/two-phase move/abort against live
//!   windows (model-level contract — the driver has no real-OS view
//!   detach; views are window-scoped);
//! - window move via Win+Arrow snap chords → `Moved` events and
//!   measured geometry deltas;
//! - window close via Alt+F4 → `CloseRequested` → Veto / Defer /
//!   Approve choreography, and the unanswered-default-Approve path;
//! - window-owned vs application-owned job cancellation policy on
//!   window destroy;
//! - window id non-reuse on regenerate.
//!
//! Not covered here (recorded as `NotRun` elsewhere): dynamic DPI /
//! monitor changes, monitor disconnect, physical manual input,
//! Native Plugin Dock (#388/#389/Synvelis #176), consumer apps.
//!
//! Run: `cargo run -p ruviform-ui-gpui --features gpui --example real_os_multiwindow_smoke`
//!
//! Manual verification only — real windows open on the desktop and
//! OS input is injected; the cursor is restored on exit. Exit codes:
//! `0` = pass, `1` = fail, `2` = unsupported (non-Windows hosts).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::dock::{DockTarget, DockWorkspace, ViewInstanceId, WindowCloseDisposition};
use ruviform_ui::view::{
    A11yRole, A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState,
    ControlSpec, Orientation, PanelSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{
    CloseDecision, DataRegistry, DemoService, DriverEvent, Event, Intent, JobState,
    LogicalPosition, LogicalSize, MonitorInfo, UiError, UiRuntime, WindowDriver, WindowId,
    WindowLifecycle, WindowSpec,
};
use ruviform_ui_gpui::os_input::{self, OsKey};
use ruviform_ui_gpui::{COMMAND_INTENT_OP, GpuiDriver, OsWindowGeometry, ViewMount};
use serde_json::{Value, json};

/// Whole-run budget — many phases of real-input waits.
const TIMEOUT: Duration = Duration::from_secs(300);
/// Driver tick cadence.
const TICK: Duration = Duration::from_millis(100);
/// Ticks each phase waits for injected input to take effect.
const SETTLE_TICKS: u64 = 15;
/// Ticks waiting for focus/close transitions after an injection.
const EVENT_WAIT_TICKS: u64 = 50;
/// Tolerance for normalized value comparisons (keyboard step = 0.05).
const VALUE_TOLERANCE: f64 = 0.03;

/// Phase machine: action phases do their work once in `enter()` and
/// immediately advance; wait phases idle in `tick()` until their
/// deadline or a recorded condition, then advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    // -- setup / focus ----------------------------------------------------
    Create,     // A+B view windows
    WaitReady,  // both Ready
    Settle,     // paint settle
    Measure,    // geometry/bounds/monitors/capabilities
    ClickB,     // real click on bravo
    WaitFocusB, // focused[B]==true
    ClickA,     // real click on alpha
    WaitFocusA, // focused[A]==true && focused[B]==false
    ApiChecks,  // focus release/unknown honesty checks
    // -- keyboard commits / state ----------------------------------------
    KeysTabA,         // Tab -> gain focused
    WaitTabASettle,   // settle -> End
    WaitKeysA,        // wait intents -> eval End commit
    UpdateSpecA,      // update_view(alpha, same spec)
    WaitUpdateSettle, // settle -> Left
    WaitKeysA2,       // wait intents -> eval Left commit
    // -- dock model ops ---------------------------------------------------
    DockOps, // detach/reattach/move/abort
    // -- jobs ------------------------------------------------------------
    Jobs,     // spawn window + app jobs
    WaitJobs, // settle
    // -- move via Win+Arrow ----------------------------------------------
    ClickMoveA,    // foreground alpha
    WaitClickMove, // settle -> Win+Left
    WaitSnapLeft,  // sample geom -> Win+Right
    WaitSnapRight, // sample geom -> Win+Down
    WaitSnapDown,  // sample geom -> focus-API restore (Win+Down minimizes)
    WaitRestore,   // sample geom -> eval
    EvalMove,
    // -- close bravo: Veto/Defer/Approve via Alt+F4 ----------------------
    ClickCloseB,
    WaitClickCloseB, // -> Alt+F4 #1
    WaitB1,          // CloseRequested#1 (Veto) seen, alive -> Alt+F4 #2
    WaitB2,          // CloseRequested#2 (Defer) seen, alive -> Alt+F4 #3
    WaitDestroyedB,  // Destroyed(B)
    EvalJobs,
    // -- regenerate while alpha stays alive (the app quits when its
    //    last window closes, so charlie must exist before alpha dies)
    Regen, // create charlie
    WaitRegenReady,
    MeasureC,
    ClickC,     // real click -> charlie foreground
    WaitFocusC, // focused[C]==true && OS fg==charlie (bounded retry)
    KeysTabC,
    WaitTabCSettle, // settle -> Right
    WaitKeysC,      // wait intents -> eval fresh state
    EvalRegen,
    // -- close alpha: unanswered -> default approve; window job cancel -
    ClickCloseA,
    WaitClickCloseA, // -> Alt+F4
    WaitDestroyedA,  // Destroyed(A) -> job drain wait
    WaitJobDrain,
    DockReattach, // views from dead windows reattach to charlie
    // -- teardown ---------------------------------------------------------
    ClickCloseC,
    WaitClickCloseC, // -> Alt+F4
    WaitDestroyedC,
}

struct Check {
    name: &'static str,
    ok: bool,
    detail: String,
}

struct State {
    win_a: Option<WindowId>,
    win_b: Option<WindowId>,
    win_c: Option<WindowId>,
    view_a: ViewInstanceId,
    view_b: ViewInstanceId,
    view_c: ViewInstanceId,
    dock: DockWorkspace,
    runtime: UiRuntime,
    epochs: AttachEpochAllocator,
    ready: BTreeSet<WindowId>,
    focused: BTreeMap<WindowId, bool>,
    // Windows observed `Focused(true)` at least once — the `focused`
    // map only keeps the latest value, which legitimately ends false
    // for whichever window was clicked last.
    focused_true: BTreeSet<WindowId>,
    traces: BTreeMap<WindowId, Vec<&'static str>>,
    // (window, intent) pairs — Action events are per-window, so
    // keyboard-commit checks must attribute intents to their window.
    intents: Vec<(WindowId, Intent)>,
    job_events: Vec<(Option<WindowId>, JobState)>,
    job_progress: usize,
    checks: Vec<Check>,
    failures: Vec<String>,
    phase: Phase,
    phase_deadline: u64,
    ticks: u64,
    started: Instant,
    // Measured records.
    geom_a0: Option<OsWindowGeometry>,
    geom_a_left: Option<OsWindowGeometry>,
    geom_a_right: Option<OsWindowGeometry>,
    geom_a_down: Option<OsWindowGeometry>,
    geom_a_down_err: Option<String>,
    geom_a_up: Option<OsWindowGeometry>,
    geom_a_up_err: Option<String>,
    geom_c: Option<OsWindowGeometry>,
    moved_a: u32,
    monitors: Vec<MonitorInfo>,
    capabilities: Option<ruviform_ui::WindowCapabilities>,
    last_gain_commit: Option<f64>,
    b_close_count: u32,
    window_jobs_cancelled: usize,
    // Injected-click activation is occasionally dropped by the OS —
    // bounded retries keep a flaky click from failing the whole lane.
    click_b_tries: u32,
    click_a_tries: u32,
    charlie_click_tries: u32,
    // Close clicks are gated on the OS foreground oracle the same way —
    // Alt+F4 must reach the intended window.
    close_b_tries: u32,
    close_a_tries: u32,
    close_c_tries: u32,
    receipts: Vec<Value>,
    out: Option<PathBuf>,
    revision: Option<String>,
    cursor_home: Option<(i32, i32)>,
    // Test windows raised to the topmost z-band so real clicks reach
    // them even when a covering window (e.g. the agent IDE) owns the
    // desktop foreground. Titles are re-resolved and lowered in
    // `finish`.
    raised: Vec<String>,
    // OS-level foreground oracle: title of `GetForegroundWindow`
    // observed after each injected click — independent of GPUI's own
    // activation events.
    fg_log: Vec<String>,
    // Most recent OS foreground title recorded by `click_at` — gates
    // the focus wait phases on the independent OS oracle.
    last_fg: Option<String>,
    result: Option<Result<(), String>>,
}

fn main() -> ExitCode {
    let opts = parse_args();
    let meta = collect_meta(&opts);
    if !os_input::supported() {
        let doc = unsupported_doc(&meta);
        if let Err(e) = write_verdict(&doc, opts.out.as_deref()) {
            eprintln!("[rf243-multiwin] failed to write verdict: {e}");
            eprintln!("{doc}");
        }
        eprintln!("[rf243-multiwin] verdict: unsupported");
        return ExitCode::from(2);
    }
    let cursor_home = os_input::cursor_position()
        .map_err(|e| eprintln!("[rf243-multiwin] cursor_position probe failed: {e}"))
        .ok();
    run_smoke(&opts, cursor_home)
}

struct Opts {
    out: Option<PathBuf>,
    revision: Option<String>,
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
                "real_os_multiwindow_smoke — Issue #243 real-OS multiwindow smoke (Windows)\n\
                 \n\
                 Drives 2-3 real GPUI windows through SendInput OS input:\n\
                 per-window focus, Win+Arrow move, Alt+F4 close\n\
                 (Veto/Defer/Approve/default), keyboard value commits,\n\
                 update_view state survival, Dock detach/reattach/move,\n\
                 window-vs-app job cancellation, regenerate + id non-reuse.\n\
                 \n\
                 --out <path>       verdict JSON destination\n\
                 --revision <rev>   revision override\n\
                 \n\
                 Exit codes: 0 pass, 1 fail, 2 unsupported (non-Windows)."
            );
            std::process::exit(0);
        }
    }
    Opts { out, revision }
}

fn collect_meta(opts: &Opts) -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "revision": opts
            .revision
            .clone()
            .or_else(|| option_env!("RF243_MULTIWIN_REVISION").map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string()),
    })
}

fn unsupported_doc(meta: &Value) -> Value {
    json!({
        "schema": "rf243-real-os-multiwindow-smoke/v1",
        "issue": 243,
        "lane": "windows-sendinput-multiwindow",
        "input_method": "win32-SendInput keyboard/pointer + example-local modifier chords (Win+Arrow, Alt+F4)",
        "input_method_note": "OS automation injection through the real OS input path — not physical manual input; distinct from GPUI simulated input. The real cursor is moved for focus clicks and restored on exit.",
        "verdict": "unsupported",
        "reasons": [format!(
            "os_input: real OS input injection is implemented for Windows only (os={})",
            std::env::consts::OS
        )],
        "checks": [],
        "environment": meta,
        "run": Value::Null,
    })
}

// ---------------------------------------------------------------------------
// View spec — one Panel with a single Fader so Tab traversal has exactly
// one stop and keyboard commits are deterministic.
// ---------------------------------------------------------------------------

fn key(name: &str) -> ComponentKey {
    ComponentKey(name.to_string())
}

fn fader_control() -> ControlSpec {
    ControlSpec {
        key: key("gain"),
        control_id: None,
        target: Some("mixer.gain".to_string()),
        descriptor: ValueDescriptor {
            default_value: Some(0.5),
            step: Some(0.05),
            ..ValueDescriptor::ranged(0.0, 1.0)
        },
        orientation: Orientation::Vertical,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Slider),
            label: Some("Gain".to_string()),
            orientation: Some(Orientation::Vertical),
            value_min: Some(0.0),
            value_max: Some(1.0),
            value_text: None,
        },
    }
}

fn view_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: key("root"),
            title: Some("RF243 multiwindow smoke".to_string()),
            children: vec![ComponentSpec::Fader(fader_control())],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn mount_host(st: &mut State, view_id: ViewInstanceId) -> ViewHost {
    let mut host = ViewHost::new(view_id, st.epochs.mint());
    host.mount(view_spec()).expect("mount smoke view");
    host
}

fn view_id(raw: u64) -> ViewInstanceId {
    ViewInstanceId::new(raw).expect("nonzero view instance")
}

// ---------------------------------------------------------------------------
// OS input helpers — `os_input` covers unmodified keys and pointer;
// modifier chords (Win+Arrow snap, Alt+F4 close) go through a small
// example-local SendInput batch. Still OS automation injection.
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[expect(unsafe_code)]
fn chord(vks: &[u16]) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    };

    fn ki(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    // Chord = modifiers down, key down, key up, modifiers up (reverse).
    let (mods, key) = vks.split_at(vks.len() - 1);
    let key = key[0];
    let mut inputs: Vec<INPUT> = Vec::with_capacity(vks.len() * 2);
    for &m in mods {
        inputs.push(ki(m, 0));
    }
    inputs.push(ki(key, 0));
    inputs.push(ki(key, KEYEVENTF_KEYUP));
    for &m in mods.iter().rev() {
        inputs.push(ki(m, KEYEVENTF_KEYUP));
    }
    let expected = u32::try_from(inputs.len()).map_err(|e| e.to_string())?;
    // SAFETY: `inputs` is a fully initialized slice that outlives the
    // call; SendInput reads exactly len*size_of::<INPUT>() bytes.
    let sent = unsafe {
        SendInput(
            expected,
            inputs.as_ptr(),
            i32::try_from(size_of::<INPUT>()).unwrap_or(i32::MAX),
        )
    };
    if sent == expected {
        Ok(())
    } else {
        Err(format!("SendInput chord injected {sent}/{expected} events"))
    }
}

#[cfg(not(windows))]
fn chord(_vks: &[u16]) -> Result<(), String> {
    Err("modifier chords unsupported on this OS".to_string())
}

const VK_LWIN: u16 = 0x5B;
const VK_LEFT: u16 = 0x25;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_MENU: u16 = 0x12;
const VK_F4: u16 = 0x73;

/// Top-level HWND whose window title exactly matches `title`.
#[cfg(windows)]
#[expect(unsafe_code)]
fn hwnd_by_title(title: &str) -> Option<usize> {
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` is NUL-terminated and outlives the call; a null
    // class name matches a top-level window of any class by title.
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    (!hwnd.is_null()).then_some(hwnd as usize)
}

/// Raises `title`'s window into the topmost z-band so injected clicks
/// land on it even when another application covers the desktop. This
/// is test-harness window management only — the focus/close behavior
/// under test is still produced by real OS input.
#[cfg(windows)]
#[expect(unsafe_code)]
fn raise_topmost(title: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    let Some(hwnd) = hwnd_by_title(title) else {
        return false;
    };
    // SAFETY: `hwnd` is a live top-level window found by title; the
    // call only moves it into the topmost band — NOMOVE|NOSIZE keep
    // geometry and NOACTIVATE keeps the current foreground.
    unsafe {
        SetWindowPos(
            hwnd as _,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        ) != 0
    }
}

#[cfg(not(windows))]
fn raise_topmost(_title: &str) -> bool {
    false
}

#[cfg(windows)]
#[expect(unsafe_code)]
fn lower_topmost(title: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    let Some(hwnd) = hwnd_by_title(title) else {
        return;
    };
    // SAFETY: `hwnd` is a live top-level window found by title; the
    // call only drops the topmost flag — NOMOVE|NOSIZE|NOACTIVATE keep
    // geometry and foreground untouched.
    unsafe {
        SetWindowPos(
            hwnd as _,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(windows))]
fn lower_topmost(_title: &str) {}

/// Restores every window this run raised back to the normal z-band.
/// Destroyed windows resolve to no HWND and are skipped.
fn restore_topmost(st: &mut State) {
    for title in st.raised.drain(..) {
        lower_topmost(&title);
    }
}

/// Title of the top-level window under screen point `(x, y)` — the
/// hit-test oracle for where an injected click actually lands.
#[cfg(windows)]
#[expect(unsafe_code)]
fn title_at_point(x: i32, y: i32) -> String {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GetAncestor, GetWindowTextW, WindowFromPoint,
    };
    // SAFETY: `WindowFromPoint` takes a POINT by value; the returned
    // HWND is queried read-only. GA_ROOT walks to the top-level owner.
    unsafe {
        let pt = POINT { x, y };
        let mut hwnd: HWND = WindowFromPoint(pt);
        if hwnd.is_null() {
            return String::new();
        }
        hwnd = GetAncestor(hwnd, GA_ROOT);
        if hwnd.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(
            hwnd,
            buf.as_mut_ptr(),
            i32::try_from(buf.len()).unwrap_or(i32::MAX),
        );
        String::from_utf16_lossy(&buf[..usize::try_from(n.max(0)).unwrap_or(0)])
    }
}

#[cfg(not(windows))]
fn title_at_point(_x: i32, _y: i32) -> String {
    String::new()
}

/// Title of the current OS foreground window, or `""`.
#[cfg(windows)]
#[expect(unsafe_code)]
fn foreground_title() -> String {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    // SAFETY: `GetForegroundWindow` returns a possibly-null HWND that
    // `GetWindowTextW` accepts; `buf` is a valid out-buffer.
    unsafe {
        let fg: HWND = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        if fg.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(
            fg,
            buf.as_mut_ptr(),
            i32::try_from(buf.len()).unwrap_or(i32::MAX),
        );
        String::from_utf16_lossy(&buf[..usize::try_from(n.max(0)).unwrap_or(0)])
    }
}

#[cfg(not(windows))]
fn foreground_title() -> String {
    String::new()
}

fn click_at(x: i32, y: i32, st: &mut State) {
    use std::thread::sleep;
    // Small gaps so the OS sees a real down/up pair, not a merged event.
    if let Err(e) = os_input::move_to(x, y) {
        st.failures.push(format!("os_input move_to: {e:?}"));
    }
    sleep(Duration::from_millis(40));
    let under = title_at_point(x, y);
    // Cursor read-back — the click lands wherever the OS cursor
    // actually is. `under` is hit-test math at the commanded point,
    // not proof the cursor arrived (same oracle as the #313 pointer
    // smoke).
    let cursor = os_input::cursor_position().ok();
    for r in [os_input::mouse_down(), os_input::mouse_up()] {
        if let Err(e) = r {
            st.failures.push(format!("os_input click: {e:?}"));
        }
    }
    sleep(Duration::from_millis(400));
    // OS-level foreground oracle — independent of GPUI's activation
    // events. Records which top-level window actually took the click.
    let fg = foreground_title();
    st.fg_log.push(format!(
        "click({x},{y})->cursor={cursor:?} under={under:?} foreground={fg:?}"
    ));
    st.last_fg = Some(fg.clone());
    eprintln!(
        "[rf243-multiwin] click ({x},{y}) cursor={cursor:?} under={under:?} -> foreground={fg:?}"
    );
}

fn press(key: OsKey, st: &mut State) {
    if let Err(e) = os_input::press_key(key) {
        st.failures.push(format!("press_key {}: {e:?}", key.name()));
    }
}

fn chord_send(vks: &[u16], st: &mut State) {
    if let Err(e) = chord(vks) {
        st.failures.push(format!("chord {vks:?}: {e}"));
    }
}

/// Client-area point near the top — panel title/padding zone, inert
/// for value controls but inside the window so the click activates it.
fn focus_point(geom: &OsWindowGeometry) -> (i32, i32) {
    (
        geom.origin_x + i32::try_from(geom.width).unwrap_or(i32::MAX) / 2,
        geom.origin_y + (i32::try_from(geom.height).unwrap_or(i32::MAX) / 8).max(8),
    )
}

// ---------------------------------------------------------------------------
// Intent inspection
// ---------------------------------------------------------------------------

/// `commit` intents on the `gain` control of `window`, normalized.
fn gain_commits(intents: &[(WindowId, Intent)], window: WindowId) -> Vec<f64> {
    intents
        .iter()
        .filter(|(w, _)| *w == window)
        .filter_map(|(_, i)| {
            if i.op != COMMAND_INTENT_OP {
                return None;
            }
            let inner = &i.params["intent"];
            if inner["kind"].as_str()? != "commit" {
                return None;
            }
            if inner["control_id"].as_str()? != "gain" {
                return None;
            }
            inner["value"]["normalized"].as_f64()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Checks / phase plumbing
// ---------------------------------------------------------------------------

fn record(st: &mut State, name: &'static str, ok: bool, detail: impl Into<String>) {
    st.checks.push(Check {
        name,
        ok,
        detail: detail.into(),
    });
}

fn lifecycle_kind(event: &WindowLifecycle) -> Option<&'static str> {
    Some(match event {
        WindowLifecycle::Created => "Created",
        WindowLifecycle::Ready => "Ready",
        WindowLifecycle::Resized { .. } => "Resized",
        WindowLifecycle::Moved { .. } => "Moved",
        WindowLifecycle::Focused(_) => "Focused",
        WindowLifecycle::ScaleFactorChanged { .. } => "ScaleFactorChanged",
        WindowLifecycle::CloseRequested => "CloseRequested",
        WindowLifecycle::Destroyed => "Destroyed",
        _ => return None,
    })
}

fn set_phase(st: &mut State, phase: Phase) {
    st.phase = phase;
    st.phase_deadline = st.ticks
        + match phase {
            Phase::WaitReady | Phase::WaitRegenReady => 600,
            Phase::WaitFocusB
            | Phase::WaitFocusA
            | Phase::WaitFocusC
            | Phase::WaitClickMove
            | Phase::WaitClickCloseB
            | Phase::WaitClickCloseA
            | Phase::WaitClickCloseC
            | Phase::WaitB1
            | Phase::WaitB2
            | Phase::WaitDestroyedB
            | Phase::WaitDestroyedA
            | Phase::WaitDestroyedC
            | Phase::WaitRestore => EVENT_WAIT_TICKS,
            Phase::WaitJobDrain => SETTLE_TICKS * 3,
            _ => SETTLE_TICKS * 2,
        };
    eprintln!("[rf243-multiwin] phase -> {phase:?} @tick {}", st.ticks);
}

fn fail(st: &mut State, msg: impl Into<String>) {
    st.failures.push(msg.into());
    finish(st);
}

fn finish(st: &mut State) {
    if st.result.is_none() {
        restore_topmost(st);
        st.result = Some(final_verdict(st));
    }
}

fn is_destroyed(st: &State, w: WindowId) -> bool {
    st.traces
        .get(&w)
        .is_some_and(|t| t.last() == Some(&"Destroyed"))
}

// ---------------------------------------------------------------------------
// Phase entry actions — run once per phase transition (wait phases are
// no-ops here; their work happens in `tick` at the deadline).
// ---------------------------------------------------------------------------

fn enter(st: &mut State, drv: &mut dyn WindowDriver, query: &GpuiDriver, phase: Phase) {
    match phase {
        Phase::Create => enter_create(st, query),
        Phase::Measure => enter_measure(st, drv, query),
        Phase::ClickB => {
            st.click_b_tries += 1;
            click_window(
                st,
                query,
                st.win_b.expect("win_b"),
                "bravo",
                Phase::WaitFocusB,
                true,
            );
        }
        Phase::ClickA => {
            st.click_a_tries += 1;
            // Give the OS a beat between the two focus clicks — rapid
            // back-to-back SendInput clicks can leave the second one
            // without foreground rights.
            std::thread::sleep(Duration::from_millis(700));
            click_window(
                st,
                query,
                st.win_a.expect("win_a"),
                "alpha",
                Phase::WaitFocusA,
                true,
            );
        }
        Phase::ApiChecks => enter_api_checks(st, drv),
        Phase::KeysTabA => {
            press(OsKey::Tab, st);
            set_phase(st, Phase::WaitTabASettle);
        }
        Phase::UpdateSpecA => enter_update_spec_a(st, query),
        Phase::DockOps => enter_dock_ops(st),
        Phase::Jobs => enter_jobs(st),
        Phase::ClickMoveA => {
            click_window(
                st,
                query,
                st.win_a.expect("win_a"),
                "alpha",
                Phase::WaitClickMove,
                false,
            );
        }
        Phase::EvalMove => enter_eval_move(st),
        Phase::ClickCloseB => {
            st.close_b_tries += 1;
            click_window(
                st,
                query,
                st.win_b.expect("win_b"),
                "bravo",
                Phase::WaitClickCloseB,
                false,
            );
        }
        Phase::ClickCloseA => {
            st.close_a_tries += 1;
            click_window(
                st,
                query,
                st.win_a.expect("win_a"),
                "alpha",
                Phase::WaitClickCloseA,
                false,
            );
        }
        Phase::EvalJobs => set_phase(st, Phase::Regen),
        Phase::Regen => enter_regen(st, query),
        Phase::MeasureC => enter_measure_c(st, query),
        Phase::ClickC => {
            st.charlie_click_tries += 1;
            click_window(
                st,
                query,
                st.win_c.expect("win_c"),
                "charlie",
                Phase::WaitFocusC,
                true,
            );
        }
        Phase::DockReattach => enter_dock_reattach(st),
        Phase::KeysTabC => {
            press(OsKey::Tab, st);
            set_phase(st, Phase::WaitTabCSettle);
        }
        Phase::EvalRegen => enter_eval_regen(st),
        Phase::ClickCloseC => {
            st.close_c_tries += 1;
            click_window(
                st,
                query,
                st.win_c.expect("win_c"),
                "charlie",
                Phase::WaitClickCloseC,
                false,
            );
        }
        _ => {}
    }
}

/// Inject a real click at `win`'s focus point, then advance to `next`.
/// `abort_on_err` decides whether a geometry failure ends the run
/// (`fail` — focus clicks need a live window) or is recorded and the
/// phase still advances (move/close clicks tolerate a minimized or
/// closing window).
fn click_window(
    st: &mut State,
    query: &GpuiDriver,
    win: WindowId,
    label: &str,
    next: Phase,
    abort_on_err: bool,
) {
    let what = match next {
        Phase::WaitClickMove => " before move click",
        Phase::WaitClickCloseB | Phase::WaitClickCloseA | Phase::WaitClickCloseC => {
            " before close click"
        }
        _ => "",
    };
    match query.window_geometry(win) {
        Ok(g) => {
            let (x, y) = focus_point(&g);
            eprintln!("[rf243-multiwin] click {label} at ({x},{y})");
            click_at(x, y, st);
        }
        Err(e) => {
            let msg = format!("window_geometry({label}){what}: {e:?}");
            if abort_on_err {
                fail(st, msg);
            } else {
                st.failures.push(msg);
            }
        }
    }
    set_phase(st, next);
}

fn enter_update_spec_a(st: &mut State, query: &GpuiDriver) {
    let a = st.win_a.expect("win_a");
    match query.update_view(a, view_spec()) {
        Ok(()) => {
            eprintln!("[rf243-multiwin] update_view(alpha) queued");
            set_phase(st, Phase::WaitUpdateSettle);
        }
        Err(e) => fail(st, format!("update_view: {e:?}")),
    }
}

fn enter_create(st: &mut State, query: &GpuiDriver) {
    let mut q = query.clone();
    for (title, view, pos) in [
        ("rf243 alpha", st.view_a, LogicalPosition { x: 80, y: 80 }),
        ("rf243 bravo", st.view_b, LogicalPosition { x: 820, y: 80 }),
    ] {
        let host = mount_host(st, view);
        match q.create_view_window(
            &WindowSpec {
                title: title.to_string(),
                logical_size: LogicalSize::new(640, 480),
                position: Some(pos),
                resizable: true,
            },
            ViewMount { host, theme: None },
        ) {
            Ok(id) => {
                eprintln!("[rf243-multiwin] created {title} {id}");
                if title == "rf243 alpha" {
                    st.win_a = Some(id);
                } else {
                    st.win_b = Some(id);
                }
                st.dock.register_window(id);
            }
            Err(e) => fail(st, format!("create {title}: {e:?}")),
        }
    }
    set_phase(st, Phase::WaitReady);
}

fn enter_measure(st: &mut State, drv: &mut dyn WindowDriver, query: &GpuiDriver) {
    let a = st.win_a.expect("win_a");
    st.geom_a0 = query.window_geometry(a).ok();
    st.monitors = drv.monitors();
    st.capabilities = Some(drv.capabilities());
    let bounds_a = query.component_bounds(a);
    let has_gain = bounds_a
        .as_ref()
        .is_ok_and(|m| m.contains_key(&key("gain")));
    record(
        st,
        "component_bounds_and_geometry",
        st.geom_a0.is_some() && has_gain,
        format!(
            "geom_a0={:?} gain_bound={has_gain} monitors={}",
            st.geom_a0
                .as_ref()
                .map(|g| (g.origin_x, g.origin_y, g.width, g.height)),
            st.monitors.len()
        ),
    );
    // The agent IDE window can cover the whole desktop and
    // keep foreground — raise the test windows so the real
    // clicks below actually reach them.
    for title in ["rf243 alpha", "rf243 bravo"] {
        if raise_topmost(title) {
            st.raised.push(title.to_string());
        } else {
            st.failures
                .push(format!("raise_topmost({title}) found no window"));
        }
    }
    set_phase(st, Phase::ClickB);
}

fn enter_api_checks(st: &mut State, drv: &mut dyn WindowDriver) {
    let b = st.win_b.expect("win_b");
    let rel = drv.focus(b, false);
    record(
        st,
        "focus_release_unsupported",
        matches!(rel, Err(UiError::Unsupported { .. })),
        format!("focus(bravo,false) -> {rel:?}"),
    );
    let unknown = WindowId::new(u64::MAX).expect("nonzero");
    let unk = drv.focus(unknown, true);
    record(
        st,
        "focus_unknown_window_host_error",
        matches!(unk, Err(UiError::Host { .. })),
        format!("focus(unknown,true) -> {unk:?}"),
    );
    if let Some(c) = &st.capabilities {
        record(
            st,
            "capabilities_declared",
            format!("{c:?}").contains("multi_window: Supported"),
            format!("{c:?}"),
        );
    }
    set_phase(st, Phase::KeysTabA);
}

fn enter_dock_ops(st: &mut State) {
    use std::fmt::Write as _;
    // Model-level Dock contract against the two live windows —
    // detach/reattach, two-phase move commit + abort rollback.
    // The driver has no real-OS view detach; this verifies the
    // workspace bookkeeping consumers run alongside.
    let (a, b) = (st.win_a.expect("win_a"), st.win_b.expect("win_b"));
    let (v1, v2) = (st.view_a, st.view_b);
    let mut ok = true;
    let mut detail = String::new();
    let (owner_v1, owner_v2);
    {
        let ws = &mut st.dock;
        macro_rules! step {
            ($r:expr, $what:expr) => {
                if let Err(e) = $r {
                    ok = false;
                    let _ = write!(detail, "{}: {:?}; ", $what, e);
                }
            };
        }
        // WaitReady already attached both views to their
        // windows — attach only fills any gap so the detach/
        // reattach/move steps below exercise real transitions.
        for (v, w) in [(v1, a), (v2, b)] {
            if ws.owner_of(v).is_none() {
                step!(ws.attach_view(v, DockTarget::Window(w)), "attach");
            }
        }
        step!(ws.detach_view(v1), "detach v1");
        ok &= ws.is_detached(v1);
        let _ = write!(detail, "detached_v1={} ", ws.is_detached(v1));
        step!(ws.attach_view(v1, DockTarget::Window(a)), "reattach v1->A");
        ok &= ws.owner_of(v1) == Some(a);
        match ws.begin_move(v1, DockTarget::Window(b)) {
            Ok(token) => {
                ok &= ws.is_transferring(v1);
                step!(ws.abort_move(token), "abort move v1");
            }
            Err(e) => {
                ok = false;
                let _ = write!(detail, "begin_move v1: {e:?}; ");
            }
        }
        ok &= ws.owner_of(v1) == Some(a);
        match ws.move_view(v2, DockTarget::Window(a)) {
            Ok(_) => ok &= ws.owner_of(v2) == Some(a),
            Err(e) => {
                ok = false;
                let _ = write!(detail, "move v2->A: {e:?}; ");
            }
        }
        match ws.move_view(v2, DockTarget::Window(b)) {
            Ok(_) => ok &= ws.owner_of(v2) == Some(b),
            Err(e) => {
                ok = false;
                let _ = write!(detail, "move v2->B: {e:?}; ");
            }
        }
        owner_v1 = ws.owner_of(v1);
        owner_v2 = ws.owner_of(v2);
    }
    record(
        st,
        "dock_detach_reattach_two_phase_move",
        ok,
        format!("owner_v1={owner_v1:?} owner_v2={owner_v2:?} {detail}"),
    );
    set_phase(st, Phase::Jobs);
}

fn enter_jobs(st: &mut State) {
    let a = st.win_a.expect("win_a");
    let win_receipts = st.runtime.dispatch_window_intent(
        a,
        Intent::new("job-win", "demo.sleep", json!({ "ms": 120_000 })),
    );
    let app_events = st
        .runtime
        .dispatch_op("job-app", "demo.sleep", json!({ "ms": 120_000 }));
    for re in &win_receipts {
        st.receipts.push(json!({
            "route": format!("{:?}", re.window),
            "event": format!("{:?}", re.event),
        }));
    }
    for ev in &app_events {
        st.receipts.push(json!({ "event": format!("{ev:?}") }));
    }
    eprintln!(
        "[rf243-multiwin] jobs spawned: window-owned on {a}, app-level; has_running={}",
        st.runtime.has_running_jobs()
    );
    set_phase(st, Phase::WaitJobs);
}

fn enter_eval_move(st: &mut State) {
    let moved = st.moved_a;
    let changed_left = match (&st.geom_a0, &st.geom_a_left) {
        (Some(a), Some(b)) => a.origin_x != b.origin_x || a.origin_y != b.origin_y,
        _ => false,
    };
    let changed_right = match (&st.geom_a_left, &st.geom_a_right) {
        (Some(a), Some(b)) => a.origin_x != b.origin_x || a.origin_y != b.origin_y,
        _ => false,
    };
    record(
        st,
        "window_move_real_input",
        moved >= 1 && (changed_left || changed_right),
        format!(
            "moved_events={moved} g0={:?} left={:?} right={:?} down={:?} up={:?}",
            st.geom_a0.as_ref().map(|g| (g.origin_x, g.origin_y)),
            st.geom_a_left.as_ref().map(|g| (g.origin_x, g.origin_y)),
            st.geom_a_right.as_ref().map(|g| (g.origin_x, g.origin_y)),
            st.geom_a_down.as_ref().map(|g| (g.origin_x, g.origin_y)),
            st.geom_a_up.as_ref().map(|g| (g.origin_x, g.origin_y)),
        ),
    );
    // Win+Down minimizes a restored window: geometry becomes
    // unavailable (iconic) and Win+Up restores it. Both halves
    // are real-OS observations, not driver calls.
    record(
        st,
        "window_minimize_restore_real_input",
        st.geom_a_down.is_none() && st.geom_a_up.is_some(),
        format!(
            "down_geom={:?} down_err={:?} up_geom={:?} up_err={:?}",
            st.geom_a_down
                .as_ref()
                .map(|g| (g.origin_x, g.origin_y, g.width, g.height)),
            st.geom_a_down_err,
            st.geom_a_up
                .as_ref()
                .map(|g| (g.origin_x, g.origin_y, g.width, g.height)),
            st.geom_a_up_err,
        ),
    );
    set_phase(st, Phase::ClickCloseB);
}

fn enter_regen(st: &mut State, query: &GpuiDriver) {
    let mut q = query.clone();
    let host = mount_host(st, st.view_c);
    match q.create_view_window(
        &WindowSpec {
            title: "rf243 charlie".to_string(),
            logical_size: LogicalSize::new(640, 480),
            position: Some(LogicalPosition { x: 400, y: 300 }),
            resizable: true,
        },
        ViewMount { host, theme: None },
    ) {
        Ok(id) => {
            st.win_c = Some(id);
            st.dock.register_window(id);
            eprintln!("[rf243-multiwin] regenerated charlie {id}");
            set_phase(st, Phase::WaitRegenReady);
        }
        Err(e) => fail(st, format!("create charlie: {e:?}")),
    }
}

fn enter_measure_c(st: &mut State, query: &GpuiDriver) {
    let c = st.win_c.expect("win_c");
    st.geom_c = query.window_geometry(c).ok();
    let bounds = query.component_bounds(c);
    let has_gain = bounds.as_ref().is_ok_and(|m| m.contains_key(&key("gain")));
    let fresh_id = st.win_a != Some(c) && st.win_b != Some(c);
    record(
        st,
        "regenerate_fresh_window_id_bounds",
        fresh_id && has_gain && st.geom_c.is_some(),
        format!("charlie={c} fresh_id={fresh_id} gain_bound={has_gain}"),
    );
    if raise_topmost("rf243 charlie") {
        st.raised.push("rf243 charlie".to_string());
    } else {
        st.failures
            .push("raise_topmost(rf243 charlie) found no window".to_string());
    }
    set_phase(st, Phase::ClickC);
}

fn enter_dock_reattach(st: &mut State) {
    use std::fmt::Write as _;
    // ViewInstanceIds survived window death in the detached
    // pool — reattach them to the regenerated window.
    let c = st.win_c.expect("win_c");
    let (vb, vc, va) = (st.view_b, st.view_c, st.view_a);
    let mut ok = true;
    let mut detail = String::new();
    let (owner_b, owner_c, detached_a);
    {
        let ws = &mut st.dock;
        for v in [vb, vc] {
            if let Err(e) = ws.attach_view(v, DockTarget::Window(c)) {
                ok = false;
                let _ = write!(detail, "attach {v:?}: {e:?}; ");
            }
        }
        ok &= ws.owner_of(vb) == Some(c);
        ok &= ws.owner_of(vc) == Some(c);
        // view_a was detached when alpha closed — it stays pooled.
        ok &= ws.is_detached(va);
        owner_b = ws.owner_of(vb);
        owner_c = ws.owner_of(vc);
        detached_a = ws.is_detached(va);
    }
    let _ = write!(
        detail,
        "owner_b={owner_b:?} owner_c={owner_c:?} detached_a={detached_a}"
    );
    record(st, "dock_view_ownership_survives_regen", ok, detail);
    set_phase(st, Phase::ClickCloseC);
}

fn enter_eval_regen(st: &mut State) {
    // Fresh host → default 0.5 → Right commits ≈0.55. If view
    // state had leaked from alpha the commit would be ≈1.0.
    let c = st.win_c.expect("win_c");
    let commits = gain_commits(&st.intents, c);
    let last = commits.last().copied();
    let ok = last.is_some_and(|n| (n - 0.55).abs() <= VALUE_TOLERANCE);
    record(
        st,
        "regenerated_view_state_is_fresh",
        ok,
        format!("last_gain_commit={last:?} expected≈0.55 (default 0.5 + step 0.05)"),
    );
    set_phase(st, Phase::ClickCloseA);
}

// ---------------------------------------------------------------------------
// Event handling
// ---------------------------------------------------------------------------

fn on_windowed(
    st: &mut State,
    drv: &mut dyn WindowDriver,
    window: WindowId,
    event: WindowLifecycle,
) {
    if let Some(kind) = lifecycle_kind(&event) {
        st.traces.entry(window).or_default().push(kind);
    }
    match event {
        WindowLifecycle::Ready => {
            st.ready.insert(window);
        }
        WindowLifecycle::Focused(f) => {
            st.focused.insert(window, f);
            if f {
                st.focused_true.insert(window);
            }
        }
        WindowLifecycle::Moved { .. } => {
            if Some(window) == st.win_a {
                st.moved_a += 1;
            }
        }
        WindowLifecycle::Action(intent) => {
            st.intents.push((window, intent));
        }
        WindowLifecycle::CloseRequested => {
            if Some(window) == st.win_b {
                st.b_close_count += 1;
                let decision = match st.b_close_count {
                    1 => CloseDecision::Veto,
                    2 => CloseDecision::Defer,
                    _ => CloseDecision::Approve,
                };
                eprintln!(
                    "[rf243-multiwin] bravo CloseRequested #{} -> {decision:?}",
                    st.b_close_count
                );
                if let Err(e) = drv.respond_close(window, decision) {
                    fail(st, format!("respond_close bravo: {e:?}"));
                }
            }
            // alpha/charlie: deliberately unanswered — default approve.
        }
        WindowLifecycle::Destroyed => {
            if Some(window) == st.win_b {
                if let Err(e) = st
                    .dock
                    .remove_window(window, WindowCloseDisposition::DetachViews)
                {
                    st.failures.push(format!("remove_window bravo: {e:?}"));
                }
            } else if Some(window) == st.win_a {
                st.window_jobs_cancelled = st.runtime.cancel_window_jobs(window);
                eprintln!(
                    "[rf243-multiwin] alpha destroyed; cancelled {} window jobs",
                    st.window_jobs_cancelled
                );
                if let Err(e) = st
                    .dock
                    .remove_window(window, WindowCloseDisposition::DetachViews)
                {
                    st.failures.push(format!("remove_window alpha: {e:?}"));
                }
                set_phase(st, Phase::WaitJobDrain);
            } else if Some(window) == st.win_c {
                finish(st);
            }
        }
        _ => {}
    }
}

/// Per-tick phase evaluation — waits, assertions, transitions.
fn tick(st: &mut State, drv: &mut dyn WindowDriver, query: &GpuiDriver) {
    st.ticks += 1;
    for re in st.runtime.drain_jobs_routed() {
        if let Event::Job { state, .. } = re.event {
            // Progress ticks would flood the evidence — count them and
            // keep only state transitions (Finished/Cancelled/Failed).
            if state == JobState::Progress {
                st.job_progress += 1;
            } else {
                st.job_events.push((re.window, state));
            }
        }
    }
    if st.result.is_some() || st.started.elapsed() > TIMEOUT {
        finish(st);
        return;
    }
    let deadline_hit = st.ticks >= st.phase_deadline;
    tick_boot(st, deadline_hit);
    tick_alpha_input(st, deadline_hit);
    tick_move(st, drv, query, deadline_hit);
    tick_close_b(st, deadline_hit);
    tick_regen_alpha(st, deadline_hit);
    tick_close_c(st, deadline_hit);
}

/// Boot waits: both windows `Ready`, then paint settle.
fn tick_boot(st: &mut State, deadline_hit: bool) {
    match st.phase {
        Phase::WaitReady => {
            if let (Some(a), Some(b)) = (st.win_a, st.win_b) {
                if st.ready.contains(&a) && st.ready.contains(&b) {
                    // Dock attach happens once both windows exist.
                    for (v, w) in [(st.view_a, a), (st.view_b, b)] {
                        if st.dock.owner_of(v).is_none() {
                            let _ = st.dock.attach_view(v, DockTarget::Window(w));
                        }
                    }
                    set_phase(st, Phase::Settle);
                } else if deadline_hit {
                    finish(st);
                }
            } else if deadline_hit {
                finish(st);
            }
        }
        Phase::Settle if deadline_hit => set_phase(st, Phase::Measure),
        _ => {}
    }
}

/// Alpha-window waits: focus clicks, keyboard commits, `update_view`.
fn tick_alpha_input(st: &mut State, deadline_hit: bool) {
    match st.phase {
        Phase::WaitFocusB => {
            // GPUI focus AND the independent OS foreground oracle must
            // both agree the click landed on bravo.
            if st.focused.get(&st.win_b.expect("win_b")) == Some(&true)
                && st.last_fg.as_deref() == Some("rf243 bravo")
            {
                set_phase(st, Phase::ClickA);
            } else if deadline_hit {
                if st.click_b_tries < 3 {
                    eprintln!("[rf243-multiwin] bravo focus click missed — retrying");
                    set_phase(st, Phase::ClickB);
                } else {
                    finish(st);
                }
            }
        }
        Phase::WaitFocusA => {
            if st.focused.get(&st.win_a.expect("win_a")) == Some(&true)
                && st.focused.get(&st.win_b.expect("win_b")) == Some(&false)
                && st.last_fg.as_deref() == Some("rf243 alpha")
            {
                set_phase(st, Phase::ApiChecks);
            } else if deadline_hit {
                if st.click_a_tries < 3 {
                    eprintln!("[rf243-multiwin] alpha focus click missed — retrying");
                    set_phase(st, Phase::ClickA);
                } else {
                    finish(st);
                }
            }
        }
        Phase::WaitTabASettle => {
            if deadline_hit {
                press(OsKey::End, st);
                set_phase(st, Phase::WaitKeysA);
            }
        }
        Phase::WaitKeysA => {
            if deadline_hit {
                let a = st.win_a.expect("win_a");
                let commits = gain_commits(&st.intents, a);
                let last = commits.last().copied();
                st.last_gain_commit = last;
                let ok = last.is_some_and(|n| (n - 1.0).abs() <= VALUE_TOLERANCE);
                record(
                    st,
                    "keyboard_commit_end_max",
                    ok,
                    format!("alpha gain commits={commits:?} expected≈1.0 (End)"),
                );
                set_phase(st, Phase::UpdateSpecA);
            }
        }
        Phase::WaitUpdateSettle => {
            if deadline_hit {
                press(OsKey::Left, st);
                set_phase(st, Phase::WaitKeysA2);
            }
        }
        Phase::WaitKeysA2 if deadline_hit => {
            let a = st.win_a.expect("win_a");
            let commits = gain_commits(&st.intents, a);
            let last = commits.last().copied();
            let expected = st.last_gain_commit.map(|p| p - 0.05);
            let ok = match (last, expected) {
                (Some(n), Some(e)) => (n - e).abs() <= VALUE_TOLERANCE,
                _ => false,
            };
            record(
                st,
                "view_state_survives_update_view",
                ok,
                format!(
                    "prev={:?} last={last:?} expected≈{expected:?} (Left -0.05)",
                    st.last_gain_commit
                ),
            );
            set_phase(st, Phase::DockOps);
        }
        _ => {}
    }
}

/// Move waits: job spawn settle, Win+Arrow snap sequence, restore.
fn tick_move(st: &mut State, drv: &mut dyn WindowDriver, query: &GpuiDriver, deadline_hit: bool) {
    match st.phase {
        Phase::WaitJobs => {
            if deadline_hit {
                set_phase(st, Phase::ClickMoveA);
            }
        }
        Phase::WaitClickMove => {
            if deadline_hit {
                chord_send(&[VK_LWIN, VK_LEFT], st);
                set_phase(st, Phase::WaitSnapLeft);
            }
        }
        Phase::WaitSnapLeft => {
            if deadline_hit {
                st.geom_a_left = query.window_geometry(st.win_a.expect("win_a")).ok();
                chord_send(&[VK_LWIN, VK_RIGHT], st);
                set_phase(st, Phase::WaitSnapRight);
            }
        }
        Phase::WaitSnapRight => {
            if deadline_hit {
                st.geom_a_right = query.window_geometry(st.win_a.expect("win_a")).ok();
                chord_send(&[VK_LWIN, VK_DOWN], st);
                set_phase(st, Phase::WaitSnapDown);
            }
        }
        Phase::WaitSnapDown => {
            if deadline_hit {
                // Win+Down minimizes a restored window — geometry is
                // legitimately unavailable while iconic. Record both
                // the value and the backend's error. No real-input
                // chord can retarget the minimized window (Win+Arrow
                // goes to whatever took foreground), so restore via
                // the driver's focus API — setup only; the close
                // choreography below stays real-input.
                let a = st.win_a.expect("win_a");
                match query.window_geometry(a) {
                    Ok(g) => st.geom_a_down = Some(g),
                    Err(e) => st.geom_a_down_err = Some(format!("{e:?}")),
                }
                if let Err(e) = drv.focus(a, true) {
                    st.failures
                        .push(format!("focus(alpha) restore after minimize: {e:?}"));
                }
                set_phase(st, Phase::WaitRestore);
            }
        }
        Phase::WaitRestore if deadline_hit => {
            let a = st.win_a.expect("win_a");
            match query.window_geometry(a) {
                Ok(g) => st.geom_a_up = Some(g),
                Err(e) => st.geom_a_up_err = Some(format!("{e:?}")),
            }
            set_phase(st, Phase::EvalMove);
        }
        _ => {}
    }
}

/// Bravo close waits: Alt+F4 Veto/Defer/Approve choreography.
fn tick_close_b(st: &mut State, deadline_hit: bool) {
    match st.phase {
        Phase::WaitClickCloseB => {
            if deadline_hit {
                // Alt+F4 must reach bravo — if the close click did not
                // take foreground, re-click (bounded) before sending it.
                if st.last_fg.as_deref() != Some("rf243 bravo") && st.close_b_tries < 3 {
                    eprintln!("[rf243-multiwin] bravo not foreground before Alt+F4 — re-clicking");
                    set_phase(st, Phase::ClickCloseB);
                } else {
                    chord_send(&[VK_MENU, VK_F4], st);
                    set_phase(st, Phase::WaitB1);
                }
            }
        }
        Phase::WaitB1 => {
            // CloseRequested#1 auto-responds Veto — window must stay.
            if st.b_close_count >= 1 && !is_destroyed(st, st.win_b.expect("win_b")) {
                set_phase(st, Phase::WaitB2);
                chord_send(&[VK_MENU, VK_F4], st);
            } else if deadline_hit {
                finish(st);
            }
        }
        Phase::WaitB2 => {
            // CloseRequested#2 auto-responds Defer — window must stay.
            if st.b_close_count >= 2 && !is_destroyed(st, st.win_b.expect("win_b")) {
                set_phase(st, Phase::WaitDestroyedB);
                chord_send(&[VK_MENU, VK_F4], st);
            } else if deadline_hit {
                finish(st);
            }
        }
        Phase::WaitDestroyedB => {
            if is_destroyed(st, st.win_b.expect("win_b")) {
                set_phase(st, Phase::EvalJobs);
            } else if deadline_hit {
                finish(st);
            }
        }
        _ => {}
    }
}

/// Post-bravo waits: charlie focus/keys, alpha close + job drain,
/// charlie readiness.
fn tick_regen_alpha(st: &mut State, deadline_hit: bool) {
    match st.phase {
        Phase::WaitFocusC => {
            let c = st.win_c.expect("win_c");
            if st.focused.get(&c) == Some(&true) && st.last_fg.as_deref() == Some("rf243 charlie") {
                set_phase(st, Phase::KeysTabC);
            } else if deadline_hit {
                if st.charlie_click_tries < 3 {
                    eprintln!("[rf243-multiwin] charlie focus click missed — retrying");
                    set_phase(st, Phase::ClickC);
                } else {
                    finish(st);
                }
            }
        }
        Phase::WaitClickCloseA => {
            if deadline_hit {
                if st.last_fg.as_deref() != Some("rf243 alpha") && st.close_a_tries < 3 {
                    eprintln!("[rf243-multiwin] alpha not foreground before Alt+F4 — re-clicking");
                    set_phase(st, Phase::ClickCloseA);
                } else {
                    chord_send(&[VK_MENU, VK_F4], st);
                    set_phase(st, Phase::WaitDestroyedA);
                }
            }
        }
        Phase::WaitDestroyedA => {
            if st.win_a.is_some_and(|a| is_destroyed(st, a)) {
                set_phase(st, Phase::WaitJobDrain);
            } else if deadline_hit {
                finish(st);
            }
        }
        Phase::WaitJobDrain => {
            if deadline_hit {
                set_phase(st, Phase::DockReattach);
            }
        }
        Phase::WaitRegenReady => {
            if st.win_c.is_some_and(|c| st.ready.contains(&c)) {
                set_phase(st, Phase::MeasureC);
            } else if deadline_hit {
                finish(st);
            }
        }
        Phase::WaitTabCSettle => {
            if deadline_hit {
                press(OsKey::Right, st);
                set_phase(st, Phase::WaitKeysC);
            }
        }
        Phase::WaitKeysC if deadline_hit => set_phase(st, Phase::EvalRegen),
        _ => {}
    }
}

/// Charlie close waits: Alt+F4 then `Destroyed` → verdict.
fn tick_close_c(st: &mut State, deadline_hit: bool) {
    match st.phase {
        Phase::WaitClickCloseC => {
            if deadline_hit {
                if st.last_fg.as_deref() != Some("rf243 charlie") && st.close_c_tries < 3 {
                    eprintln!(
                        "[rf243-multiwin] charlie not foreground before Alt+F4 — re-clicking"
                    );
                    set_phase(st, Phase::ClickCloseC);
                } else {
                    chord_send(&[VK_MENU, VK_F4], st);
                    set_phase(st, Phase::WaitDestroyedC);
                }
            }
        }
        Phase::WaitDestroyedC if st.win_c.is_some_and(|c| is_destroyed(st, c)) || deadline_hit => {
            finish(st);
        }
        _ => {}
    }
}

fn on_event(st: &mut State, ev: DriverEvent, drv: &mut dyn WindowDriver, query: &GpuiDriver) {
    // One-shot phase entry actions — loop so action phases that chain
    // into other action phases all run before the event is handled.
    // Wait phases are no-ops in `enter`, so the loop always terminates.
    for _ in 0..64 {
        let p = st.phase;
        enter(st, drv, query, p);
        if st.phase == p || st.result.is_some() {
            break;
        }
    }
    if st.result.is_some() {
        return;
    }
    match ev {
        DriverEvent::Tick => tick(st, drv, query),
        DriverEvent::Windowed { window, event } => on_windowed(st, drv, window, event),
        _ => {}
    }
    // A recorded verdict ends the GPUI loop — otherwise `run` blocks
    // forever and the process never exits.
    if st.result.is_some() {
        drv.exit();
    }
}

fn run_smoke(opts: &Opts, cursor_home: Option<(i32, i32)>) -> ExitCode {
    let mut driver = GpuiDriver::new();
    eprintln!("[rf243-multiwin] capabilities: {:?}", driver.capabilities());

    let state = Rc::new(RefCell::new(State {
        win_a: None,
        win_b: None,
        win_c: None,
        view_a: view_id(1),
        view_b: view_id(2),
        view_c: view_id(3),
        dock: DockWorkspace::new(),
        runtime: UiRuntime::new(DataRegistry::new()),
        epochs: AttachEpochAllocator::new(),
        ready: BTreeSet::new(),
        focused: BTreeMap::new(),
        focused_true: BTreeSet::new(),
        traces: BTreeMap::new(),
        intents: Vec::new(),
        job_events: Vec::new(),
        job_progress: 0,
        checks: Vec::new(),
        failures: Vec::new(),
        phase: Phase::Create,
        phase_deadline: 0,
        ticks: 0,
        started: Instant::now(),
        geom_a0: None,
        geom_a_left: None,
        geom_a_right: None,
        geom_a_down: None,
        geom_a_down_err: None,
        geom_a_up: None,
        geom_a_up_err: None,
        geom_c: None,
        moved_a: 0,
        monitors: Vec::new(),
        capabilities: None,
        last_gain_commit: None,
        b_close_count: 0,
        window_jobs_cancelled: 0,
        click_b_tries: 0,
        click_a_tries: 0,
        charlie_click_tries: 0,
        close_b_tries: 0,
        close_a_tries: 0,
        close_c_tries: 0,
        receipts: Vec::new(),
        out: opts.out.clone(),
        revision: opts.revision.clone(),
        cursor_home,
        raised: Vec::new(),
        fg_log: Vec::new(),
        last_fg: None,
        result: None,
    }));

    {
        let mut st = state.borrow_mut();
        if let Err(e) = st.runtime.register_service(DemoService::new()) {
            eprintln!("[rf243-multiwin] register_service failed: {e:?}");
        }
    }

    let handler = state.clone();
    let query = driver.clone();
    driver
        .run(Some(TICK), move |event, drv| {
            let mut st = handler.borrow_mut();
            on_event(&mut st, event, drv, &query);
        })
        .expect("gpui driver run");

    if let Some(home) = state.borrow().cursor_home {
        let _ = os_input::move_to(home.0, home.1);
    }

    // The GPUI app quits when its last window closes — if every window
    // died before the phase machine reached `finish` (e.g. an
    // unexpected early close), `run` returns with no verdict. Record
    // the fail verdict anyway so evidence is never lost.
    {
        let mut st = state.borrow_mut();
        if st.result.is_none() {
            finish(&mut st);
        }
    }

    let st = state.borrow();
    match &st.result {
        Some(Ok(())) => ExitCode::SUCCESS,
        Some(Err(_)) => ExitCode::FAILURE,
        None => {
            eprintln!("[rf243-multiwin] run returned without a verdict");
            ExitCode::FAILURE
        }
    }
}

/// Aggregate named checks + failures into the verdict.
fn final_verdict(st: &mut State) -> Result<(), String> {
    if let (Some(a), Some(b)) = (st.win_a, st.win_b) {
        // Each window must have been observed Focused(true) by GPUI
        // AND taken OS foreground by a real click (the fg oracle).
        let fg_alpha = st
            .fg_log
            .iter()
            .any(|l| l.contains("foreground=\"rf243 alpha\""));
        let fg_bravo = st
            .fg_log
            .iter()
            .any(|l| l.contains("foreground=\"rf243 bravo\""));
        let focus_ok = st.focused_true.contains(&a)
            && st.focused_true.contains(&b)
            && fg_alpha
            && fg_bravo
            && st.traces.get(&a).is_some_and(|t| t.contains(&"Ready"))
            && st.traces.get(&b).is_some_and(|t| t.contains(&"Ready"));
        record(
            st,
            "per_window_focus_real_click",
            focus_ok,
            format!(
                "focused_true={:?} focused={:?} fg_alpha={fg_alpha} fg_bravo={fg_bravo}",
                st.focused_true, st.focused
            ),
        );
        let b_trace = st.traces.get(&b).cloned().unwrap_or_default();
        let close_reqs = b_trace.iter().filter(|&&k| k == "CloseRequested").count();
        let close_ok =
            close_reqs >= 3 && b_trace.last() == Some(&"Destroyed") && st.b_close_count >= 3;
        record(
            st,
            "close_veto_defer_approve_real_input",
            close_ok,
            format!("bravo trace={b_trace:?} close_count={}", st.b_close_count),
        );
        let a_trace = st.traces.get(&a).cloned().unwrap_or_default();
        let a_close_ok =
            a_trace.contains(&"CloseRequested") && a_trace.last() == Some(&"Destroyed");
        record(
            st,
            "close_default_approve_real_input",
            a_close_ok,
            format!("alpha trace={a_trace:?}"),
        );
        // Job policy: window-owned job cancelled on destroy; the
        // app-level job (owner None) must not be cancelled.
        let win_cancelled = st
            .job_events
            .iter()
            .any(|(w, s)| *w == Some(a) && *s == JobState::Cancelled);
        let app_cancelled = st
            .job_events
            .iter()
            .any(|(w, s)| w.is_none() && *s == JobState::Cancelled);
        record(
            st,
            "window_job_cancelled_app_job_survives",
            st.window_jobs_cancelled > 0 && win_cancelled && !app_cancelled,
            format!(
                "cancelled_count={} win_event={win_cancelled} app_cancelled={app_cancelled} \
                 app_still_running={} progress_events={} events={:?}",
                st.window_jobs_cancelled,
                st.runtime.has_running_jobs(),
                st.job_progress,
                st.job_events
            ),
        );
    }

    let failed_checks: Vec<&str> = st.checks.iter().filter(|c| !c.ok).map(|c| c.name).collect();
    let mut problems = st.failures.clone();
    for c in &failed_checks {
        problems.push(format!("check failed: {c}"));
    }

    let doc = verdict_doc(st, &problems);
    if let Err(e) = write_verdict(&doc, st.out.as_deref()) {
        eprintln!("[rf243-multiwin] failed to write verdict file: {e}");
    }
    eprintln!("[rf243-multiwin] verdict: {}", doc["verdict"]);

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

fn verdict_doc(st: &State, problems: &[String]) -> Value {
    let checks: Vec<Value> = st
        .checks
        .iter()
        .map(|c| json!({"name": c.name, "ok": c.ok, "detail": c.detail}))
        .collect();
    let geom = |g: &Option<OsWindowGeometry>| {
        g.as_ref()
            .map(|g| json!({"x": g.origin_x, "y": g.origin_y, "w": g.width, "h": g.height}))
    };
    json!({
        "schema": "rf243-real-os-multiwindow-smoke/v1",
        "issue": 243,
        "lane": "windows-sendinput-multiwindow",
        "input_method": "win32-SendInput keyboard/pointer + example-local modifier chords (Win+Arrow, Alt+F4)",
        "input_method_note": "OS automation injection through the real OS input path — not physical manual input; distinct from GPUI simulated input. The real cursor is moved for focus clicks and restored on exit.",
        "verdict": if problems.is_empty() { "pass" } else { "fail" },
        "problems": problems,
        "checks": checks,
        "traces": st.traces.iter().map(|(w, t)| (format!("{w:?}"), t.clone())).collect::<BTreeMap<_, _>>(),
        "focused": st.focused.iter().map(|(w, f)| (format!("{w:?}"), *f)).collect::<BTreeMap<_, _>>(),
        "os_foreground_log": st.fg_log,
        "intents": st.intents.iter().map(|(w, i)| json!({
            "window": format!("{w:?}"),
            "id": i.id, "op": i.op,
            "kind": i.params["intent"]["kind"],
            "control_id": i.params["intent"]["control_id"],
            "normalized": i.params["intent"]["value"]["normalized"],
        })).collect::<Vec<_>>(),
        "job_events": st.job_events.iter().map(|(w, s)| json!({
            "window": w.map(|x| format!("{x:?}")), "state": format!("{s:?}"),
        })).collect::<Vec<_>>(),
        "receipts": st.receipts,
        "geometry": {
            "alpha_initial": st.geom_a0.as_ref().map(|g| json!({"x": g.origin_x, "y": g.origin_y, "w": g.width, "h": g.height, "scale": g.scale_factor})),
            "alpha_after_winleft": geom(&st.geom_a_left),
            "alpha_after_winright": geom(&st.geom_a_right),
            "alpha_after_windown": geom(&st.geom_a_down),
            "alpha_after_windown_err": st.geom_a_down_err,
            "alpha_after_winup": geom(&st.geom_a_up),
            "alpha_after_winup_err": st.geom_a_up_err,
            "charlie": st.geom_c.as_ref().map(|g| json!({"x": g.origin_x, "y": g.origin_y, "w": g.width, "h": g.height, "scale": g.scale_factor})),
        },
        "moved_events_alpha": st.moved_a,
        "monitors": st.monitors.iter().map(|m| json!({
            "name": m.name, "x": m.x, "y": m.y, "w": m.width, "h": m.height, "scale": m.scale_factor,
        })).collect::<Vec<_>>(),
        "capabilities": st.capabilities.as_ref().map(|c| format!("{c:?}")),
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "revision": st.revision.clone().unwrap_or_else(|| "unknown".to_string()),
        },
        "run": {
            "ticks": st.ticks,
            "elapsed_ms": u64::try_from(st.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    })
}

fn write_verdict(doc: &Value, out: Option<&std::path::Path>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, &text).map_err(|e| e.to_string())?;
        eprintln!("[rf243-multiwin] verdict written to {}", path.display());
    } else {
        println!("{text}");
    }
    Ok(())
}
