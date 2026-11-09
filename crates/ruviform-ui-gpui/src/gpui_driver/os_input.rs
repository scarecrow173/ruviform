//! OS-level keyboard injection for real-OS smoke verification
//! (Issue #313, RF-GT-04 Windows partial scope).
//!
//! [`press_key`] injects one full keystroke (key-down + key-up) into
//! the OS input stream — Win32 `SendInput` on Windows. This is OS
//! automation injection: it enters through the same foreground-window
//! input path physical input takes, but it is *not* physical manual
//! input, and evidence must keep that distinction. It is also a
//! different layer from the GPUI test helpers
//! (`gpui_driver::observe::dispatch_input`, `simulate_keystrokes`),
//! which never leave the toolkit — those results stay
//! simulated-input evidence only.
//!
//! Only the keys the Issue #479 keyboard convention consumes are
//! exposed. Platforms without an implementation return
//! [`UiError::Unsupported`] — capability honesty, same policy as
//! `ime`/`native_surface_slot`.
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
/// `false` means [`press_key`] would return [`UiError::Unsupported`]
/// — callers use this to classify a run as unsupported instead of
/// attempting injection that cannot work.
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

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    };

    use super::{OsKey, UiError};

    #[expect(unsafe_code)]
    pub(super) fn press(key: OsKey) -> Result<(), UiError> {
        let inputs = [
            keyboard_input(key.vk(), 0),
            keyboard_input(key.vk(), KEYEVENTF_KEYUP),
        ];
        // SAFETY: `inputs` is a valid two-element array of fully
        // initialized `INPUT` records that outlives the call;
        // `SendInput` reads exactly `cinputs * cbSize` bytes from it
        // and does not retain the pointer.
        let sent = unsafe {
            SendInput(
                2,
                inputs.as_ptr(),
                i32::try_from(size_of::<INPUT>()).unwrap_or(i32::MAX),
            )
        };
        if sent == 2 {
            Ok(())
        } else {
            Err(UiError::host(format!(
                "SendInput injected {sent}/2 events for key {}",
                key.name()
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
}
