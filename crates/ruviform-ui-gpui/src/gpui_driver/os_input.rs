//! OS-level input injection for real-OS smoke verification
//! (Issue #313, RF-GT-04 Windows partial scope).
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
//! Only the keys the Issue #479 keyboard convention consumes are
//! exposed, and pointer injection covers absolute move plus the left
//! button only — no chords, modifiers, or other buttons. Platforms
//! without an implementation return [`UiError::Unsupported`] —
//! capability honesty, same policy as `ime`/`native_surface_slot`.
//!
//! **RT contract:** non-RT only — test/manual-verification path,
//! never on the RT path.

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

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    use super::{OsKey, UiError};

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

    /// Absolute screen position → `SendInput` normalized
    /// `MOUSEEVENTF_ABSOLUTE` coordinates. With `VIRTUALDESK` the
    /// 0..=65535 space spans the whole virtual screen whose origin
    /// may be negative — normalize against `GetSystemMetrics` rather
    /// than assuming the primary monitor.
    #[expect(unsafe_code)]
    fn normalized(x: i32, y: i32) -> Result<(i32, i32), UiError> {
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
    use super::{OsKey, UiError};

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
