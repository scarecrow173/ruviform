//! OS-level input injection for real-OS smoke verification
//! (Issue #313, RF-GT-04 Windows partial scope; Issue #552 adds the
//! injection-effect preflight).
//!
//! [`press_key`] injects one full keystroke (key-down + key-up) and
//! [`move_to`]/[`mouse_down`]/[`mouse_up`] inject absolute cursor
//! motion and left-button transitions into the OS input stream —
//! Win32 `SendInput` on Windows. This is OS automation injection:
//! it enters through the same foreground-window input path physical
//! input takes, but it is *not* physical manual input, and evidence
//! must keep that distinction. It is also a different layer from the
//! GPUI test helpers (`gpui_driver::observe::dispatch_input`,
//! `simulate_keystrokes`), which never leave the toolkit — those
//! results stay simulated-input evidence only.
//!
//! [`preflight`] is the environment gate the real-OS lanes run before
//! their bodies (Issue #552): `SendInput` can report success while
//! injected events have zero effect (observed 2026-11-10 — an
//! injected-flag filter dropped every event while the API kept
//! returning `sent == expected`). The preflight therefore verifies
//! *effect*, not acceptance: it injects a harmless probe (a small
//! cursor move that is restored, or an unassigned-key down/up) and
//! reads the result back through independent OS APIs
//! (`GetCursorPos`/`GetAsyncKeyState`). A lane that sees
//! `PreflightReport::effective == false` must report
//! `environment_blocked`, never a GUI `fail`.
//!
//! Only the keys the Issue #479 keyboard convention consumes are
//! exposed, and pointer injection covers absolute move plus the left
//! button only — no chords, modifiers, or other buttons. Platforms
//! without an implementation return [`UiError::Unsupported`] —
//! capability honesty, same policy as `ime`/`native_surface_slot`.
//!
//! **RT contract:** non-RT only — test/manual-verification path,
//! never on the RT path.

use std::time::Duration;

use ruviform_ui::UiError;

/// A keyboard key [`press_key`] can inject.
///
/// The set is deliberately limited to the keys the Issue #479
/// keyboard convention handles (Tab focus traversal plus the
/// value-control arrows/Home/End/Page keys). Modifier chords and
/// unlisted keys are out of scope for this slice.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OsKey {
    /// Tab — unmodified Tab is wired to focus traversal (Issue #451).
    Tab,
    /// Left arrow — one negative effective step.
    Left,
    /// Right arrow — one positive effective step.
    Right,
    /// Up arrow — one positive effective step.
    Up,
    /// Down arrow — one negative effective step.
    Down,
    /// Page Up — ten positive effective steps.
    PageUp,
    /// Page Down — ten negative effective steps.
    PageDown,
    /// Home — minimum value.
    Home,
    /// End — maximum value.
    End,
}

impl OsKey {
    /// Stable lowercase name for evidence records.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tab => "tab",
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
            Self::PageUp => "pageup",
            Self::PageDown => "pagedown",
            Self::Home => "home",
            Self::End => "end",
        }
    }

    #[cfg(windows)]
    fn vk(self) -> u16 {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            VK_DOWN, VK_END, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RIGHT, VK_TAB, VK_UP,
        };
        match self {
            Self::Tab => VK_TAB,
            Self::Left => VK_LEFT,
            Self::Right => VK_RIGHT,
            Self::Up => VK_UP,
            Self::Down => VK_DOWN,
            Self::PageUp => VK_PRIOR,
            Self::PageDown => VK_NEXT,
            Self::Home => VK_HOME,
            Self::End => VK_END,
        }
    }
}

/// The honest result of one OS-injection operation driven through an
/// adapter (Issue #313 — `RealOsPointerOrKeyboard` capability).
///
/// The variants deliberately mirror the lane verdict vocabulary
/// (`pass`/`unsupported`/`environment_blocked`/`not_run`) so a caller
/// never has to translate a failed-looking [`UiError`] into a verdict
/// that hides the real cause:
///
/// - [`OsInputOutcome::Injected`] — every `SendInput` event was
///   accepted by the OS; carries the measured evidence (commanded
///   target point and cursor read-back for pointer moves).
/// - [`OsInputOutcome::Unsupported`] — no injection backend on this
///   OS; the operation never attempted an impossible injection.
/// - [`OsInputOutcome::EnvironmentBlocked`] — the session preflight
///   (Issue #552) proved injection ineffective (e.g. `SendInput`
///   accepted but events never took effect). This is an environment
///   diagnosis, never a GUI implementation failure.
/// - [`OsInputOutcome::NotRun`] — a run precondition was not met
///   (window not focused, component not yet laid out); the lane may
///   retry after fixing the precondition.
///
/// Contract violations (stale target pins, unknown windows,
/// partially-accepted injections) surface through the caller's own
/// error channel instead — they are `fail`, never one of these.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum OsInputOutcome {
    /// Every injected event was accepted; carries measured evidence.
    Injected {
        /// Short action tag — `"key-press"`, `"pointer-move"`,
        /// `"mouse-down"`, `"mouse-up"`.
        action: &'static str,
        /// Commanded physical-px screen point for pointer moves;
        /// `None` for keyboard/button ops that act at the current
        /// position or foreground window.
        commanded: Option<(i32, i32)>,
        /// [`cursor_position`] read-back after the move (pointer
        /// ops only) — the evidence the lane compares against
        /// `commanded`.
        cursor: Option<(i32, i32)>,
        /// Human-readable detail (key name, timings, measured
        /// deltas) for the verdict record.
        detail: String,
    },
    /// No injection backend on this OS.
    Unsupported {
        /// Why injection is unsupported here.
        reason: String,
    },
    /// Session preflight proved injection ineffective — record
    /// `environment_blocked`, never a GUI `fail`.
    EnvironmentBlocked {
        /// The failing preflight stage's evidence.
        detail: String,
    },
    /// A run precondition was not met — `not_run` evidence; the lane
    /// may retry after fixing the precondition.
    NotRun {
        /// Which precondition failed (e.g. `"window w2 not focused"`).
        reason: String,
    },
}

/// Reports whether this platform has an OS injection backend.
///
/// `false` means [`press_key`] and the pointer functions would
/// return [`UiError::Unsupported`] — callers use this to classify a
/// run as unsupported instead of attempting injection that cannot
/// work.
#[must_use]
pub fn supported() -> bool {
    cfg!(windows)
}

/// Injects one keystroke (key-down then key-up) into the OS input
/// queue. The keystroke is delivered to whatever window the OS
/// considers foreground — callers must verify the target window is
/// focused before injecting.
///
/// # Errors
///
/// - [`UiError::Unsupported`] — the host OS has no injection backend
///   in this slice (non-Windows).
/// - [`UiError::Host`] — the OS accepted fewer than the two events
///   (down + up); a partial keystroke is never reported as success.
pub fn press_key(key: OsKey) -> Result<(), UiError> {
    imp::press(key)
}

/// Reports the cursor position in physical screen pixels
/// (virtual-desktop coordinates — negative values are valid on
/// secondary monitors left of/above the primary).
///
/// # Errors
///
/// - [`UiError::Unsupported`] — the host OS has no pointer backend
///   in this slice (non-Windows).
/// - [`UiError::Host`] — the OS position query failed.
pub fn cursor_position() -> Result<(i32, i32), UiError> {
    imp::cursor_position()
}

/// Moves the cursor to an absolute screen position in physical
/// pixels (virtual-desktop coordinates). The move is queued as one
/// `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK`
/// event — callers sequence [`mouse_down`]/[`mouse_up`] around it
/// for clicks and drags.
///
/// # Errors
///
/// - [`UiError::Unsupported`] — the host OS has no pointer backend
///   in this slice (non-Windows).
/// - [`UiError::Host`] — the virtual-screen metrics are unusable or
///   the OS accepted fewer than the one event.
pub fn move_to(x: i32, y: i32) -> Result<(), UiError> {
    imp::move_to(x, y)
}

/// Presses the left mouse button at the current cursor position.
///
/// # Errors
///
/// - [`UiError::Unsupported`] — the host OS has no pointer backend
///   in this slice (non-Windows).
/// - [`UiError::Host`] — the OS accepted fewer than the one event.
pub fn mouse_down() -> Result<(), UiError> {
    imp::mouse_button(true)
}

/// Releases the left mouse button at the current cursor position.
///
/// # Errors
///
/// - [`UiError::Unsupported`] — the host OS has no pointer backend
///   in this slice (non-Windows).
/// - [`UiError::Host`] — the OS accepted fewer than the one event.
pub fn mouse_up() -> Result<(), UiError> {
    imp::mouse_button(false)
}

// ---------------------------------------------------------------------------
// Issue #552 — injection-effect preflight.
//
// `SendInput` can accept every event while the OS drops them all
// (observed 2026-11-10). The lanes therefore verify *effect* through
// APIs independent of the injection call before running their bodies.
// ---------------------------------------------------------------------------

/// Which injection families [`preflight`] verifies.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreflightScope {
    /// Cursor (pointer) injection only — absolute move read back via
    /// the OS cursor position, then restored.
    Pointer,
    /// Keyboard injection only — an unassigned-key down read back via
    /// the OS async key state, then released.
    Keyboard,
    /// Both pointer and keyboard injection.
    PointerAndKeyboard,
}

impl PreflightScope {
    /// The family names this scope covers, in probe order
    /// (pointer before keyboard) — `PreflightCheck::family` values.
    #[must_use]
    pub fn families(self) -> &'static [&'static str] {
        match self {
            Self::Pointer => &["pointer"],
            Self::Keyboard => &["keyboard"],
            Self::PointerAndKeyboard => &["pointer", "keyboard"],
        }
    }
}

/// One family's preflight outcome.
#[derive(Debug, Clone)]
pub struct PreflightCheck {
    /// `"pointer"` or `"keyboard"`.
    pub family: &'static str,
    /// `true` when the read-back confirmed the injected event took
    /// effect — never based on the injection call's return value.
    pub effective: bool,
    /// Human-readable evidence: positions, timings, or the API error
    /// that stopped the probe.
    pub detail: String,
}

/// Result of [`preflight`].
///
/// `effective` is `true` only when every requested family's check
/// confirmed effect. A `false` report means the lane must not run —
/// callers map it to an environment-blocked verdict, distinct from a
/// GUI implementation failure.
#[derive(Debug, Clone)]
pub struct PreflightReport {
    /// `true` when all requested checks confirmed injection effect.
    pub effective: bool,
    /// Per-family outcomes in probe order (pointer before keyboard).
    pub checks: Vec<PreflightCheck>,
}

/// Read-back wait budget and poll cadence for the probes. The public
/// path uses [`ProbeTiming::DEFAULT`]; tests shrink it.
#[derive(Debug, Clone, Copy)]
struct ProbeTiming {
    /// Delay between read-back polls.
    poll: Duration,
    /// Maximum number of read-back polls per probe (~2s at the
    /// default cadence).
    attempts: u32,
}

impl ProbeTiming {
    /// ~2s budget at 25ms cadence — generous for `SendInput` → OS
    /// state propagation, still cheap for a blocked environment.
    const DEFAULT: Self = Self {
        poll: Duration::from_millis(25),
        attempts: 80,
    };
}

/// The OS seam the preflight algorithm runs against: injection plus
/// the independent read-back APIs. The Windows backend implements it
/// with Win32 calls; unit tests substitute a mock so the blocked
/// branch is verifiable without touching the real input stream.
trait Probe {
    /// Current cursor position in physical screen pixels.
    fn cursor_position(&self) -> Result<(i32, i32), UiError>;
    /// Absolute cursor move (same path as [`move_to`]).
    fn move_to(&self, x: i32, y: i32) -> Result<(), UiError>;
    /// Virtual desktop bounds `(x, y, width, height)` in physical px.
    fn virtual_screen(&self) -> Result<(i32, i32, i32, i32), UiError>;
    /// Blocks for `duration` — injectable so tests stay instant.
    fn sleep(&self, duration: Duration);
    /// One keyboard event for virtual-key code `vk`: `up == false`
    /// injects key-down, `true` injects key-up.
    fn inject_key(&self, vk: u16, up: bool) -> Result<(), UiError>;
    /// Whether `vk` currently reads as held via the OS async key
    /// state (`GetAsyncKeyState` high bit on Windows).
    fn key_down_state(&self, vk: u16) -> bool;
}

/// Verifies that OS input injection actually takes effect before a
/// real-OS lane runs its body (Issue #552).
///
/// For each requested family this injects a *harmless* probe and
/// confirms the effect through an API independent of the injection
/// call — never trusting `SendInput`'s accepted-event count:
///
/// - **pointer**: moves the cursor 16px toward the virtual-screen
///   interior (absolute move — deterministic, unlike relative moves
///   which follow pointer-acceleration settings), polls
///   `GetCursorPos` until it lands within tolerance, then restores
///   the original position.
/// - **keyboard**: presses `VK_F24` (unassigned on typical systems,
///   no modifiers) key-down, polls `GetAsyncKeyState` until the key
///   reads as held, then always sends key-up — a confirmed-down key
///   is never left held.
///
/// The report never fails with [`UiError`]: "could not verify" is
/// itself the diagnostic, surfaced as `effective == false` with the
/// failing stage in each check's `detail`.
///
/// **RT contract:** non-RT only — test/manual-verification path.
#[must_use]
pub fn preflight(scope: PreflightScope) -> PreflightReport {
    preflight_inner(&imp::OsProbe, scope, ProbeTiming::DEFAULT)
}

fn preflight_inner(
    probe: &dyn Probe,
    scope: PreflightScope,
    timing: ProbeTiming,
) -> PreflightReport {
    let mut checks = Vec::new();
    if matches!(
        scope,
        PreflightScope::Pointer | PreflightScope::PointerAndKeyboard
    ) {
        checks.push(probe_pointer(probe, timing));
    }
    if matches!(
        scope,
        PreflightScope::Keyboard | PreflightScope::PointerAndKeyboard
    ) {
        checks.push(probe_keyboard(probe, timing));
    }
    // `all()` on an empty check list would vacuously pass — an
    // unknown/empty scope must never report effective.
    let effective = !checks.is_empty() && checks.iter().all(|c| c.effective);
    PreflightReport { effective, checks }
}

fn check(family: &'static str, effective: bool, detail: String) -> PreflightCheck {
    PreflightCheck {
        family,
        effective,
        detail,
    }
}

/// Read-back tolerance in physical px — covers absolute-move
/// normalization quantization (sub-px to ~1px), while a blocked
/// injection leaves the cursor the full 16px probe distance away.
const CURSOR_TOLERANCE: i32 = 8;
/// Probe displacement per axis — large enough to defeat tolerance,
/// small enough to be visually negligible before restore.
const PROBE_DELTA: i32 = 16;
/// Virtual key used by the keyboard probe: `VK_F24` — defined by
/// Win32, unassigned on typical systems, carries no modifiers and no
/// default binding, so a stray down/up has no observable effect.
const PREFLIGHT_VK: u16 = 0x87;

/// Picks the probe target: `origin` shifted `PROBE_DELTA` px per axis
/// toward the virtual-screen interior. Each axis independently picks
/// the direction that keeps the target on-screen, so a cursor at a
/// monitor edge still gets a full-delta move. `None` only when the
/// virtual screen is too small to fit the delta on either side.
fn probe_target(origin: (i32, i32), vx: i32, vy: i32, vw: i32, vh: i32) -> Option<(i32, i32)> {
    let x_hi = vx + vw;
    let y_hi = vy + vh;
    let dx = if origin.0 + PROBE_DELTA < x_hi {
        PROBE_DELTA
    } else if origin.0 - PROBE_DELTA >= vx {
        -PROBE_DELTA
    } else {
        return None;
    };
    let dy = if origin.1 + PROBE_DELTA < y_hi {
        PROBE_DELTA
    } else if origin.1 - PROBE_DELTA >= vy {
        -PROBE_DELTA
    } else {
        return None;
    };
    Some((origin.0 + dx, origin.1 + dy))
}

fn cursor_near(actual: (i32, i32), target: (i32, i32)) -> bool {
    (actual.0 - target.0).abs() <= CURSOR_TOLERANCE
        && (actual.1 - target.1).abs() <= CURSOR_TOLERANCE
}

fn probe_pointer(probe: &dyn Probe, timing: ProbeTiming) -> PreflightCheck {
    const FAMILY: &str = "pointer";
    let origin = match probe.cursor_position() {
        Ok(p) => p,
        Err(e) => {
            return check(
                FAMILY,
                false,
                format!("cursor_position read failed before injection: {e}"),
            );
        }
    };
    let (vx, vy, vw, vh) = match probe.virtual_screen() {
        Ok(s) => s,
        Err(e) => {
            return check(
                FAMILY,
                false,
                format!("virtual screen metrics read failed: {e}"),
            );
        }
    };
    if vw <= 1 || vh <= 1 {
        return check(
            FAMILY,
            false,
            format!("virtual screen metrics unusable ({vw}x{vh})"),
        );
    }
    let Some(target) = probe_target(origin, vx, vy, vw, vh) else {
        return check(
            FAMILY,
            false,
            format!("no {PROBE_DELTA}px interior room around cursor at {origin:?}"),
        );
    };
    if let Err(e) = probe.move_to(target.0, target.1) {
        return check(FAMILY, false, format!("cursor move injection failed: {e}"));
    }
    // Read-back: the injected move must be *observed* through
    // GetCursorPos — SendInput's accepted count proves nothing.
    let mut last_read = None;
    let mut confirmed = false;
    for _ in 0..timing.attempts {
        probe.sleep(timing.poll);
        if let Ok(p) = probe.cursor_position() {
            last_read = Some(p);
            if cursor_near(p, target) {
                confirmed = true;
                break;
            }
        }
    }
    // Always attempt restore — the user's cursor must not be left
    // displaced even when the probe failed.
    let restore = probe.move_to(origin.0, origin.1);
    let restore_note = restore.as_ref().err().map_or_else(String::new, |e| {
        format!("; cursor restore to {origin:?} failed: {e}")
    });
    if confirmed {
        check(
            FAMILY,
            true,
            format!(
                "cursor {origin:?} -> {target:?} confirmed by read-back {last_read:?}{restore_note}"
            ),
        )
    } else {
        check(
            FAMILY,
            false,
            format!(
                "read-back not confirmed within {} polls: last cursor {last_read:?}, target was {target:?}{restore_note}",
                timing.attempts
            ),
        )
    }
}

fn probe_keyboard(probe: &dyn Probe, timing: ProbeTiming) -> PreflightCheck {
    const FAMILY: &str = "keyboard";
    // Key-down first; the up event is sent unconditionally below so a
    // confirmed-down key is never left held.
    let down = probe.inject_key(PREFLIGHT_VK, false);
    let mut held = false;
    if down.is_ok() {
        for _ in 0..timing.attempts {
            probe.sleep(timing.poll);
            if probe.key_down_state(PREFLIGHT_VK) {
                held = true;
                break;
            }
        }
    }
    let up = probe.inject_key(PREFLIGHT_VK, true);
    let up_note = up.as_ref().err().map_or_else(String::new, |e| {
        format!("; key-up injection failed (key may be left held): {e}")
    });
    match down {
        Err(e) => check(
            FAMILY,
            false,
            format!("key-down injection failed: {e}{up_note}"),
        ),
        Ok(()) if !held => check(
            FAMILY,
            false,
            format!(
                "read-back not confirmed within {} polls: VK_F24 never read as held{up_note}",
                timing.attempts
            ),
        ),
        Ok(()) => check(
            FAMILY,
            true,
            format!("VK_F24 down confirmed by read-back, released{up_note}"),
        ),
    }
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    use std::time::Duration;

    use super::{OsKey, Probe, UiError};

    pub(super) struct OsProbe;

    impl Probe for OsProbe {
        fn cursor_position(&self) -> Result<(i32, i32), UiError> {
            cursor_position()
        }

        fn move_to(&self, x: i32, y: i32) -> Result<(), UiError> {
            move_to(x, y)
        }

        fn virtual_screen(&self) -> Result<(i32, i32, i32, i32), UiError> {
            virtual_screen()
        }

        fn sleep(&self, duration: Duration) {
            std::thread::sleep(duration);
        }

        fn inject_key(&self, vk: u16, up: bool) -> Result<(), UiError> {
            key_event(vk, up)
        }

        fn key_down_state(&self, vk: u16) -> bool {
            key_held(vk)
        }
    }

    pub(super) fn press(key: OsKey) -> Result<(), UiError> {
        let inputs = [
            keyboard_input(key.vk(), 0),
            keyboard_input(key.vk(), KEYEVENTF_KEYUP),
        ];
        send(&inputs, 2, &format!("key {}", key.name()))
    }

    #[expect(unsafe_code)]
    pub(super) fn cursor_position() -> Result<(i32, i32), UiError> {
        let mut point = POINT::default();
        // SAFETY: `point` is a valid out-param; `GetCursorPos` only
        // writes it.
        if unsafe { GetCursorPos(&raw mut point) } == 0 {
            return Err(UiError::host("GetCursorPos failed"));
        }
        Ok((point.x, point.y))
    }

    /// Virtual desktop bounds `(x, y, width, height)` in physical
    /// pixels — the same metrics [`normalized`] uses for absolute
    /// coordinate mapping.
    #[expect(unsafe_code)]
    fn virtual_screen() -> Result<(i32, i32, i32, i32), UiError> {
        // SAFETY: `GetSystemMetrics` takes a constant index and has
        // no preconditions.
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        if vw <= 1 || vh <= 1 {
            return Err(UiError::host(format!(
                "virtual screen metrics unusable ({vw}x{vh})"
            )));
        }
        Ok((vx, vy, vw, vh))
    }

    /// Absolute screen position → `SendInput` normalized
    /// `MOUSEEVENTF_ABSOLUTE` coordinates. With `VIRTUALDESK` the
    /// 0..=65535 space spans the whole virtual screen whose origin
    /// may be negative — normalize against `GetSystemMetrics` rather
    /// than assuming the primary monitor.
    fn normalized(x: i32, y: i32) -> Result<(i32, i32), UiError> {
        let (vx, vy, vw, vh) = virtual_screen()?;
        let nx = ((i64::from(x) - i64::from(vx)) * 65_535 / i64::from(vw))
            .clamp(0, 65_535)
            .try_into()
            .unwrap_or(0);
        let ny = ((i64::from(y) - i64::from(vy)) * 65_535 / i64::from(vh))
            .clamp(0, 65_535)
            .try_into()
            .unwrap_or(0);
        Ok((nx, ny))
    }

    pub(super) fn move_to(x: i32, y: i32) -> Result<(), UiError> {
        let (nx, ny) = normalized(x, y)?;
        let inputs = [mouse_input(
            nx,
            ny,
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        )];
        send(&inputs, 1, "pointer move")
    }

    pub(super) fn mouse_button(down: bool) -> Result<(), UiError> {
        let flag = if down {
            MOUSEEVENTF_LEFTDOWN
        } else {
            MOUSEEVENTF_LEFTUP
        };
        let inputs = [mouse_input(0, 0, flag)];
        send(
            &inputs,
            1,
            if down {
                "left button down"
            } else {
                "left button up"
            },
        )
    }

    /// Single key event for an arbitrary virtual-key code — used by
    /// the Issue #552 preflight probe (`VK_F24`), which sits outside
    /// the [`OsKey`] vocabulary.
    fn key_event(vk: u16, up: bool) -> Result<(), UiError> {
        let flags = if up { KEYEVENTF_KEYUP } else { 0 };
        let inputs = [keyboard_input(vk, flags)];
        send(
            &inputs,
            1,
            &format!(
                "key event vk=0x{vk:04X}{}",
                if up { " up" } else { " down" }
            ),
        )
    }

    /// Whether `vk` currently reads as held via `GetAsyncKeyState` —
    /// the preflight's independent keyboard read-back. `GetAsyncKeyState`
    /// reflects the physical/async key state of the whole session, not
    /// the calling thread's message queue (unlike `GetKeyState`), so it
    /// observes injected events without requiring a window message loop.
    #[expect(unsafe_code)]
    fn key_held(vk: u16) -> bool {
        // SAFETY: `GetAsyncKeyState` takes a virtual-key code and has
        // no preconditions. A negative return means the high bit —
        // "key is down" — is set.
        let state = unsafe { GetAsyncKeyState(i32::from(vk)) };
        state < 0
    }

    /// One `SendInput` batch — reports a partial injection as a host
    /// error, never as success.
    #[expect(unsafe_code)]
    fn send(inputs: &[INPUT], expected: u32, what: &str) -> Result<(), UiError> {
        // SAFETY: `inputs` is a valid slice of fully initialized
        // `INPUT` records that outlives the call; `SendInput` reads
        // exactly `cinputs * cbSize` bytes and does not retain the
        // pointer.
        let sent = unsafe {
            SendInput(
                inputs.len().try_into().unwrap_or(u32::MAX),
                inputs.as_ptr(),
                i32::try_from(size_of::<INPUT>()).unwrap_or(i32::MAX),
            )
        };
        if sent == expected {
            Ok(())
        } else {
            Err(UiError::host(format!(
                "SendInput injected {sent}/{expected} events for {what}"
            )))
        }
    }

    fn keyboard_input(vk: u16, flags: u32) -> INPUT {
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

    fn mouse_input(dx: i32, dy: i32, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::time::Duration;

    use super::{OsKey, Probe, UiError};

    pub(super) struct OsProbe;

    impl Probe for OsProbe {
        fn cursor_position(&self) -> Result<(i32, i32), UiError> {
            cursor_position()
        }

        fn move_to(&self, x: i32, y: i32) -> Result<(), UiError> {
            move_to(x, y)
        }

        fn virtual_screen(&self) -> Result<(i32, i32, i32, i32), UiError> {
            Err(unsupported_pointer("virtual screen metrics"))
        }

        fn sleep(&self, duration: Duration) {
            std::thread::sleep(duration);
        }

        fn inject_key(&self, vk: u16, _up: bool) -> Result<(), UiError> {
            Err(UiError::unsupported(
                "os_input",
                format!("real OS key injection is implemented for Windows only (vk=0x{vk:04X})"),
            ))
        }

        fn key_down_state(&self, _vk: u16) -> bool {
            false
        }
    }

    pub(super) fn press(key: OsKey) -> Result<(), UiError> {
        Err(UiError::unsupported(
            "os_input",
            format!(
                "real OS keyboard injection is implemented for Windows only (key {})",
                key.name()
            ),
        ))
    }

    pub(super) fn cursor_position() -> Result<(i32, i32), UiError> {
        Err(unsupported_pointer("cursor position"))
    }

    pub(super) fn move_to(_x: i32, _y: i32) -> Result<(), UiError> {
        Err(unsupported_pointer("pointer move"))
    }

    pub(super) fn mouse_button(_down: bool) -> Result<(), UiError> {
        Err(unsupported_pointer("mouse button"))
    }

    fn unsupported_pointer(what: &str) -> UiError {
        UiError::unsupported(
            "os_input",
            format!("real OS {what} injection is implemented for Windows only"),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// Mock [`Probe`]: `*_effective` simulates whether injected events
    /// take effect at the OS level — `false` reproduces the observed
    /// `SendInput` acceptance-without-effect failure. Every injected
    /// call is still accepted (`Ok`), exactly like the real failure.
    struct MockProbe {
        cursor: Cell<(i32, i32)>,
        cursor_effective: bool,
        key_effective: bool,
        held: Cell<bool>,
        moves: Cell<u32>,
        key_events: RefCell<Vec<(u16, bool)>>,
    }

    impl MockProbe {
        fn new() -> Self {
            Self {
                cursor: Cell::new((500, 400)),
                cursor_effective: true,
                key_effective: true,
                held: Cell::new(false),
                moves: Cell::new(0),
                key_events: RefCell::new(Vec::new()),
            }
        }

        fn timing() -> ProbeTiming {
            ProbeTiming {
                poll: Duration::ZERO,
                attempts: 3,
            }
        }
    }

    impl Probe for MockProbe {
        fn cursor_position(&self) -> Result<(i32, i32), UiError> {
            Ok(self.cursor.get())
        }

        fn move_to(&self, x: i32, y: i32) -> Result<(), UiError> {
            self.moves.set(self.moves.get() + 1);
            if self.cursor_effective {
                self.cursor.set((x, y));
            }
            Ok(())
        }

        fn virtual_screen(&self) -> Result<(i32, i32, i32, i32), UiError> {
            Ok((0, 0, 1920, 1080))
        }

        fn sleep(&self, _duration: Duration) {}

        fn inject_key(&self, vk: u16, up: bool) -> Result<(), UiError> {
            self.key_events.borrow_mut().push((vk, up));
            if self.key_effective {
                self.held.set(!up);
            }
            Ok(())
        }

        fn key_down_state(&self, _vk: u16) -> bool {
            self.held.get()
        }
    }

    #[test]
    fn pointer_blocked_when_move_has_no_effect() {
        let probe = MockProbe {
            cursor_effective: false,
            ..MockProbe::new()
        };
        let report = preflight_inner(&probe, PreflightScope::Pointer, MockProbe::timing());
        assert!(!report.effective);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].family, "pointer");
        assert!(!report.checks[0].effective);
        // Probe move + restore are both attempted even when blocked.
        assert_eq!(probe.moves.get(), 2);
        // Cursor stays where it started — the probe never displaces it.
        assert_eq!(probe.cursor.get(), (500, 400));
    }

    #[test]
    fn pointer_effective_confirms_and_restores() {
        let probe = MockProbe::new();
        let report = preflight_inner(&probe, PreflightScope::Pointer, MockProbe::timing());
        assert!(report.effective);
        assert_eq!(report.checks[0].family, "pointer");
        assert!(report.checks[0].effective);
        // Restore ran: cursor is back at the origin, not the probe target.
        assert_eq!(probe.cursor.get(), (500, 400));
        assert_eq!(probe.moves.get(), 2);
    }

    #[test]
    fn keyboard_blocked_when_down_has_no_effect_but_up_is_sent() {
        let probe = MockProbe {
            key_effective: false,
            ..MockProbe::new()
        };
        let report = preflight_inner(&probe, PreflightScope::Keyboard, MockProbe::timing());
        assert!(!report.effective);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].family, "keyboard");
        assert!(!report.checks[0].effective);
        // Both events were injected — key-up is unconditional.
        assert_eq!(
            *probe.key_events.borrow(),
            vec![(PREFLIGHT_VK, false), (PREFLIGHT_VK, true)]
        );
    }

    #[test]
    fn keyboard_effective_confirms_and_releases() {
        let probe = MockProbe::new();
        let report = preflight_inner(&probe, PreflightScope::Keyboard, MockProbe::timing());
        assert!(report.effective);
        assert!(report.checks[0].effective);
        // Down was observed held, then released.
        assert!(!probe.held.get());
        assert_eq!(
            *probe.key_events.borrow(),
            vec![(PREFLIGHT_VK, false), (PREFLIGHT_VK, true)]
        );
    }

    #[test]
    fn scope_selects_only_requested_probes() {
        let probe = MockProbe::new();
        let report = preflight_inner(&probe, PreflightScope::Keyboard, MockProbe::timing());
        assert_eq!(report.checks.len(), 1);
        assert_eq!(probe.moves.get(), 0);
        assert_eq!(probe.key_events.borrow().len(), 2);

        let probe = MockProbe::new();
        let report = preflight_inner(&probe, PreflightScope::Pointer, MockProbe::timing());
        assert_eq!(report.checks.len(), 1);
        assert_eq!(probe.key_events.borrow().len(), 0);
    }

    #[test]
    fn pointer_and_keyboard_reports_both_and_ands_effective() {
        // Pointer effective, keyboard blocked → overall ineffective.
        let probe = MockProbe {
            key_effective: false,
            ..MockProbe::new()
        };
        let report = preflight_inner(
            &probe,
            PreflightScope::PointerAndKeyboard,
            MockProbe::timing(),
        );
        assert!(!report.effective);
        assert_eq!(report.checks.len(), 2);
        assert!(report.checks[0].effective);
        assert!(!report.checks[1].effective);

        let probe = MockProbe::new();
        let report = preflight_inner(
            &probe,
            PreflightScope::PointerAndKeyboard,
            MockProbe::timing(),
        );
        assert!(report.effective);
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn probe_target_prefers_interior_then_flips_at_edges() {
        // Centered: +delta on both axes.
        assert_eq!(probe_target((500, 400), 0, 0, 1920, 1080), Some((516, 416)));
        // Right/bottom edge: -delta keeps the target on-screen.
        assert_eq!(
            probe_target((1910, 1070), 0, 0, 1920, 1080),
            Some((1894, 1054))
        );
        // Negative-origin virtual screen (secondary monitor left of
        // primary): bounds are honored, not assumed zero-based.
        assert_eq!(
            probe_target((-100, -50), -1920, -1080, 3840, 2160),
            Some((-84, -34))
        );
        // Degenerate screen: too small for the delta on either side.
        assert_eq!(probe_target((5, 5), 0, 0, 10, 10), None);
    }

    #[test]
    fn readback_tolerance_accepts_sub_delta_error() {
        // Absolute-move normalization can land a pixel or two off the
        // requested target; that must still count as effective.
        assert!(cursor_near((517, 418), (516, 416)));
        assert!(!cursor_near((500, 400), (516, 416)));
    }
}
