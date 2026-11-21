//! OS-level IME composition drive + delivered-message observation for
//! real-window verification (Issue #389 — `ScenarioCapability::Ime`).
//!
//! Mirrors [`os_input`]'s contract in the IME family:
//!
//! - [`supported`] reports *backend presence* (Win32 IMM32). The claim
//!   is presence-only — an IME-absent or IME-disabled session is
//!   diagnosed per-op as [`OsImeOutcome::EnvironmentBlocked`], never
//!   silently passed.
//! - [`preflight`] probes whether an IME is installed/enabled for the
//!   target's thread — `effective == false` means the case maps to the
//!   honest `environment_blocked`, exactly the `os_input` convention.
//! - [`drive_composition`] performs the accepted-or-refused IMM32
//!   drive: `SetFocus` → `ImmGetContext` → `ImmSetOpenStatus` →
//!   `ImmSetCompositionStringW` (`SCS_SETSTR|GCS_COMPSTR`) →
//!   `ImmNotifyIME` (`CPS_COMPLETE`) → `ImmReleaseContext`. Acceptance
//!   is *not* evidence — it only means the OS took the call.
//! - [`collect_ime_messages`] is the independent oracle: a temporary
//!   `GWLP_WNDPROC` subclass on the *target window* (installed by the
//!   calling thread, removed before return — the proc lives in this
//!   binary which never unloads, and the original proc is restored on
//!   every exit path) plus a bounded same-thread `PeekMessage`/
//!   `DispatchMessage` pump. It records which `WM_IME_*`/`WM_CHAR`
//!   messages the OS actually delivered to the window — the same
//!   evidence a real IME-aware plugin editor would consume.
//! - [`drive_composition_observed`] is the lane's composed entry
//!   point — the tap is installed *before* the drive runs because the
//!   `WM_IME_*` envelope messages are delivered **synchronously** into
//!   the window's `WndProc` during the IMM32 calls (observed on this
//!   host: only the generated `WM_CHAR` flow stays queued long enough
//!   to be pumped afterwards — a tap installed post-drive misses the
//!   whole envelope).
//!
//! The drive targets a *child HWND owned by the calling thread* — the
//! real-dock lane's documented D1 thread-affinity deviation puts the
//! plugin editor child on the scenario caller thread, so its queue is
//! pumpable here. A foreign-thread target reports `NotRun` honestly
//! rather than pumping the wrong queue.
//!
//! **RT contract:** non-RT only — test/verification path. This module
//! never fabricates a composition effect: `Injected` means "the IMM32
//! calls were accepted", and only [`ImeDelivery::commit_confirmed`]
//! (OS-delivered `WM_IME_STARTCOMPOSITION` → `WM_IME_COMPOSITION`
//! carrying `GCS_RESULTSTR` → `WM_IME_ENDCOMPOSITION`, with committed
//! text observed) is pass-grade evidence.

use std::time::Duration;

use super::os_input::{PreflightCheck, PreflightReport};

/// `NI_COMPOSITIONSTR` sub-op — commit the composition (mirrors
/// `CPS_COMPLETE` in `winuser.h`/`imm.h`); kept local so the probe
/// trait stays free of OS types.
const CPS_COMPLETE_OP: u32 = 0x0001;
/// `NI_COMPOSITIONSTR` sub-op — cancel the composition
/// (`CPS_CANCEL`).
const CPS_CANCEL_OP: u32 = 0x0004;

/// `WM_IME_*`/`WM_CHAR` ids observed by the lane tap — `winuser.h`
/// values, defined locally so analysis compiles identically on every
/// target (the constants are protocol values, not backend calls).
const WM_CHAR_ID: u32 = 0x0102;
const WM_IME_STARTCOMPOSITION_ID: u32 = 0x010D;
const WM_IME_ENDCOMPOSITION_ID: u32 = 0x010E;
const WM_IME_COMPOSITION_ID: u32 = 0x010F;
const WM_IME_SETCONTEXT_ID: u32 = 0x0281;
const WM_IME_NOTIFY_ID: u32 = 0x0282;
const WM_IME_CONTROL_ID: u32 = 0x0283;
const WM_IME_COMPOSITIONFULL_ID: u32 = 0x0284;
const WM_IME_SELECT_ID: u32 = 0x0285;
const WM_IME_CHAR_ID: u32 = 0x0286;
const WM_IME_REQUEST_ID: u32 = 0x0288;
const WM_IME_KEYDOWN_ID: u32 = 0x0290;
const WM_IME_KEYUP_ID: u32 = 0x0291;
/// `GCS_RESULTSTR` bit inside a `WM_IME_COMPOSITION` `lParam` — the
/// OS signalling a committed result string exists (`imm.h`).
/// Only the Windows delivery path and the tests consume the folding
/// helpers — keep them out of non-Windows non-test builds (dead code).
#[cfg(any(windows, test))]
const GCS_RESULTSTR_BIT: isize = 0x0800;

/// Reports whether this platform has an OS-level IME composition
/// backend.
///
/// `true` only means the IMM32 machinery exists (Windows); it says
/// nothing about an IME being installed or enabled on the session —
/// that is what [`preflight`] diagnoses. `false` means
/// [`drive_composition`] returns [`OsImeOutcome::Unsupported`]
/// without attempting anything.
#[must_use]
pub fn supported() -> bool {
    cfg!(windows)
}

/// The honest result of one OS-level IME composition drive (Issue
/// #389 — `Ime` capability).
///
/// The variants deliberately mirror [`super::os_input::OsInputOutcome`]
/// (and thus the lane verdict vocabulary `pass`/`unsupported`/
/// `environment_blocked`/`not_run`) so a caller never has to invent a
/// verdict that hides the real cause:
///
/// - [`OsImeOutcome::Injected`] — `ImmSetCompositionStringW` and
///   `ImmNotifyIME(CPS_COMPLETE)` were both *accepted* by the OS.
///   Acceptance proves nothing about delivery — only the
///   [`ImeDelivery`] oracle can confirm the `WM_IME_*` sequence
///   reached the target window.
/// - [`OsImeOutcome::Unsupported`] — no IMM32 backend on this OS; no
///   drive was attempted.
/// - [`OsImeOutcome::EnvironmentBlocked`] — the session cannot
///   deliver a composition (no IME on the layout, no input context,
///   focus refused, or the IME rejected the programmatic
///   composition). An environment diagnosis, never a GUI failure.
/// - [`OsImeOutcome::NotRun`] — a run precondition was not met (dead
///   HWND, foreign-thread target, empty text); the lane may retry
///   after fixing the precondition.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum OsImeOutcome {
    /// `ImmSetCompositionStringW` + `ImmNotifyIME(CPS_COMPLETE)` both
    /// accepted — the composition/commit drive went through at API
    /// level. Delivery is *not* implied: only the delivered-message
    /// log can confirm it.
    Injected {
        /// Short action tag — `"ime-compose-commit"`.
        action: &'static str,
        /// Human-readable detail (target HWND, driven text).
        detail: String,
    },
    /// No IMM32 backend on this OS.
    Unsupported {
        /// Why the drive is unsupported here.
        reason: String,
    },
    /// The session cannot deliver a composition — an IME is absent,
    /// disabled, or refused the programmatic drive. Record
    /// `environment_blocked`, never a GUI `fail`.
    EnvironmentBlocked {
        /// The blocking stage's evidence.
        detail: String,
    },
    /// A run precondition was not met — `not_run` evidence.
    NotRun {
        /// Which precondition failed.
        reason: String,
    },
}

/// One window message the lane tap observed being dispatched into the
/// target window's `WndProc` — the delivery record, never synthesized.
/// `comp_str`/`result_str` are only populated on `WM_IME_COMPOSITION`
/// where the tap performs the *app-side* `ImmGetCompositionStringW`
/// read a real IME-aware window would do.
#[derive(Debug, Clone, PartialEq)]
pub struct ImeDeliveredMessage {
    /// The window-message id (`WM_IME_*` or `WM_CHAR`).
    pub message: u32,
    /// `wParam` as delivered.
    pub wparam: usize,
    /// `lParam` as delivered — `GCS_*` bit flags on
    /// `WM_IME_COMPOSITION`.
    pub lparam: isize,
    /// Decoded committed character for `WM_IME_CHAR`/`WM_CHAR`
    /// (`wParam` as UTF-16 code point) — evidence garnish.
    pub text: Option<char>,
    /// `GCS_COMPSTR` read inside the dispatch — the live composition
    /// string the app would see.
    pub comp_str: Option<String>,
    /// `GCS_RESULTSTR` read inside the dispatch — the committed text
    /// the app would consume.
    pub result_str: Option<String>,
}

/// What [`collect_ime_messages`] observed: the tap result plus the
/// analysed delivery flags a verdict reads. The four flags are
/// independent observations (tap ran / start seen / result-flagged
/// composition seen / end seen), not a state machine —
/// `struct_excessive_bools` is expected here.
#[expect(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct ImeDelivery {
    /// `true` when the subclass tap was installed and the caller
    /// thread's queue was actually pumped. `false` means the oracle
    /// machinery itself could not run (dead HWND, foreign-thread
    /// target, no platform backend) — distinct from "pumped and saw
    /// nothing".
    pub observed: bool,
    /// Every logged `WM_IME_*`/`WM_CHAR` in delivery order.
    pub messages: Vec<ImeDeliveredMessage>,
    /// `WM_IME_STARTCOMPOSITION` was delivered.
    pub start_composition: bool,
    /// A `WM_IME_COMPOSITION` carrying `GCS_RESULTSTR` was delivered —
    /// the OS told the window a committed result string exists.
    pub result_composition: bool,
    /// `WM_IME_ENDCOMPOSITION` was delivered.
    pub end_composition: bool,
    /// Count of `WM_IME_CHAR`/`WM_CHAR` deliveries — the committed
    /// text flow a real editor's `WndProc` would consume.
    pub committed_chars: usize,
    /// Committed text assembled from the delivered char messages —
    /// conversion-dependent evidence, never a verdict input alone.
    pub committed_text: Option<String>,
    /// First non-empty `GCS_RESULTSTR` app-side read — the strongest
    /// commit evidence.
    pub result_text: Option<String>,
    /// First non-empty `GCS_COMPSTR` app-side read — the live
    /// composition evidence.
    pub composition_text: Option<String>,
}

impl ImeDelivery {
    /// The delivery contract the `Ime` claim rests on: the OS told
    /// the window a composition started, delivered a result-flagged
    /// `WM_IME_COMPOSITION`, ended the composition — *and* committed
    /// text actually flowed (char deliveries or a non-empty app-side
    /// result read). An accepted drive without this sequence is not
    /// a pass-grade effect.
    #[must_use]
    pub fn commit_confirmed(&self) -> bool {
        self.start_composition
            && self.result_composition
            && self.end_composition
            && (self.committed_chars > 0
                || self
                    .result_text
                    .as_deref()
                    .is_some_and(|text| !text.is_empty()))
    }
}

/// Stable display name for evidence rows — `""` for unknown ids (the
/// row still records the numeric value, so nothing is hidden).
#[must_use]
pub fn message_name(message: u32) -> &'static str {
    match message {
        WM_CHAR_ID => "WM_CHAR",
        WM_IME_STARTCOMPOSITION_ID => "WM_IME_STARTCOMPOSITION",
        WM_IME_ENDCOMPOSITION_ID => "WM_IME_ENDCOMPOSITION",
        WM_IME_COMPOSITION_ID => "WM_IME_COMPOSITION",
        WM_IME_SETCONTEXT_ID => "WM_IME_SETCONTEXT",
        WM_IME_NOTIFY_ID => "WM_IME_NOTIFY",
        WM_IME_CONTROL_ID => "WM_IME_CONTROL",
        WM_IME_COMPOSITIONFULL_ID => "WM_IME_COMPOSITIONFULL",
        WM_IME_SELECT_ID => "WM_IME_SELECT",
        WM_IME_CHAR_ID => "WM_IME_CHAR",
        WM_IME_REQUEST_ID => "WM_IME_REQUEST",
        WM_IME_KEYDOWN_ID => "WM_IME_KEYDOWN",
        WM_IME_KEYUP_ID => "WM_IME_KEYUP",
        _ => "",
    }
}

/// The OS seam the drive + preflight algorithms run against — the
/// IMM32 calls plus the thread/focus queries they depend on. The
/// Windows backend implements it with Win32; unit tests substitute a
/// scripted mock so the blocked/refused/precondition branches are
/// verifiable without touching a real IME.
trait ImeProbe {
    /// `GetCurrentThreadId` — the caller's thread id.
    fn current_thread(&self) -> u32;
    /// `GetWindowThreadProcessId(hwnd)` — the target's owning thread
    /// (`0` for a dead handle).
    fn owner_thread(&self, hwnd: u64) -> u32;
    /// `SetFocus(hwnd)` then `GetFocus() == hwnd` — whether the
    /// target really took keyboard focus (the `WM_IME_*` routing
    /// precondition).
    fn focused(&self, hwnd: u64) -> bool;
    /// `ImmGetContext(hwnd)` — an opaque context token (`u64` view of
    /// `HIMC`), or `None` when the window has no input context.
    fn context(&self, hwnd: u64) -> Option<u64>;
    /// `ImmReleaseContext(hwnd, ctx)`.
    fn release_context(&self, hwnd: u64, ctx: u64);
    /// `ImmGetOpenStatus(ctx)` — the IME conversion window is open.
    fn open_status(&self, ctx: u64) -> bool;
    /// `ImmSetOpenStatus(ctx, open)` — accepted or refused.
    fn set_open_status(&self, ctx: u64, open: bool) -> bool;
    /// `ImmSetCompositionStringW(ctx, SCS_SETSTR|GCS_COMPSTR, text)` —
    /// accepted or refused.
    fn set_composition(&self, ctx: u64, text: &str) -> bool;
    /// `ImmNotifyIME(ctx, NI_COMPOSITIONSTR, op)` — accepted or
    /// refused. `op` is `CPS_COMPLETE_OP`/`CPS_CANCEL_OP`.
    fn notify(&self, ctx: u64, op: u32) -> bool;
    /// `ImmIsIME(GetKeyboardLayout(tid))` — the caller thread's active
    /// layout is an IME (a non-IME layout still hands out input
    /// contexts, so this check is what separates real IME presence
    /// from a silent no-op composition).
    fn layout_is_ime(&self) -> bool;
    /// `ImmGetDefaultIMEWnd(hwnd) != 0` — an IME UI window exists for
    /// the target's thread.
    fn default_ime_window(&self, hwnd: u64) -> bool;
}

/// Drives a real IME composition + commit into `hwnd`'s input
/// context. `text` becomes the composition string
/// (`SCS_SETSTR|GCS_COMPSTR`); `NI_COMPOSITIONSTR`/`CPS_COMPLETE`
/// commits it. Accepted calls are *not* proof of effect — pair with
/// [`collect_ime_messages`] for the delivery oracle.
///
/// Preconditions honestly reported (never silently skipped):
/// `hwnd` must be a live window owned by the **calling thread** — its
/// `WM_IME_*` queue is only pumpable there — and must be able to take
/// keyboard focus (the OS routes `WM_IME_*` to the focused window).
#[must_use]
pub fn drive_composition(hwnd: u64, text: &str) -> OsImeOutcome {
    if !supported() {
        return OsImeOutcome::Unsupported {
            reason: "OS-level IME composition drive is implemented for \
                 Windows (IMM32) only"
                .to_string(),
        };
    }
    drive_inner(&imp::OsIme, hwnd, text)
}

fn drive_inner(probe: &dyn ImeProbe, hwnd: u64, text: &str) -> OsImeOutcome {
    if text.is_empty() {
        return OsImeOutcome::NotRun {
            reason: "empty composition text — nothing to drive".to_string(),
        };
    }
    let owner = probe.owner_thread(hwnd);
    let caller = probe.current_thread();
    if owner == 0 {
        return OsImeOutcome::NotRun {
            reason: format!("target HWND {hwnd:#x} is not a live window"),
        };
    }
    if owner != caller {
        return OsImeOutcome::NotRun {
            reason: format!(
                "target HWND {hwnd:#x} is owned by thread {owner}, not the \
                 caller thread {caller} — its IME message queue cannot be \
                 driven or pumped from here"
            ),
        };
    }
    if !probe.focused(hwnd) {
        return OsImeOutcome::EnvironmentBlocked {
            detail: "SetFocus did not leave the target focused — WM_IME_* \
                 messages would route to a different window"
                .to_string(),
        };
    }
    let Some(ctx) = probe.context(hwnd) else {
        return OsImeOutcome::EnvironmentBlocked {
            detail: "ImmGetContext returned no input context — the thread \
                 has no IME association"
                .to_string(),
        };
    };
    // A closed IME may ignore composition input — open it for the
    // drive and restore the status afterwards (the caller thread's
    // HIMC is lane-owned, but leaving state flipped is still sloppy).
    let was_open = probe.open_status(ctx);
    if !was_open {
        probe.set_open_status(ctx, true);
    }
    let outcome = if !probe.set_composition(ctx, text) {
        OsImeOutcome::EnvironmentBlocked {
            detail: format!(
                "ImmSetCompositionStringW refused {text:?} — the active IME \
                 does not accept a programmatic composition"
            ),
        }
    } else if !probe.notify(ctx, CPS_COMPLETE_OP) {
        // Never leave a half-driven composition behind — best-effort
        // cancel before reporting the block.
        probe.notify(ctx, CPS_CANCEL_OP);
        OsImeOutcome::EnvironmentBlocked {
            detail: "composition accepted but ImmNotifyIME(CPS_COMPLETE) \
                 refused — the commit could not be driven"
                .to_string(),
        }
    } else {
        OsImeOutcome::Injected {
            action: "ime-compose-commit",
            detail: format!(
                "composition {text:?} set + CPS_COMPLETE accepted on HWND \
                 {hwnd:#x} — acceptance is not delivery; the WM_IME_* log \
                 is the oracle"
            ),
        }
    };
    if !was_open {
        probe.set_open_status(ctx, was_open);
    }
    probe.release_context(hwnd, ctx);
    outcome
}

/// Probes whether an IME is installed/enabled for `hwnd`'s thread —
/// the `os_input::preflight` convention: `effective == false` means
/// the environment cannot deliver IME composition and the case must
/// record `environment_blocked`, never run the drive and guess.
///
/// Checks (family `"ime_*"`):
///
/// - `ime_layout` — `ImmIsIME(GetKeyboardLayout(caller))`: the active
///   layout is an IME. A non-IME layout still produces input contexts
///   — without this check a silent no-op drive would look plausible.
/// - `ime_context` — `ImmGetContext(hwnd)` yields an input context.
/// - `ime_ui` — `ImmGetDefaultIMEWnd(hwnd)` exists: the IME is
///   enabled for the target's thread (its UI window exists).
///
/// **RT contract:** non-RT only — test/verification path.
#[must_use]
pub fn preflight(hwnd: u64) -> PreflightReport {
    if !supported() {
        return PreflightReport {
            effective: false,
            checks: vec![PreflightCheck {
                family: "ime_backend",
                effective: false,
                detail: "IMM32 IME backend exists on Windows only".to_string(),
            }],
        };
    }
    preflight_inner(&imp::OsIme, hwnd)
}

fn preflight_inner(probe: &dyn ImeProbe, hwnd: u64) -> PreflightReport {
    let mut checks = Vec::new();
    checks.push(PreflightCheck {
        family: "ime_layout",
        effective: probe.layout_is_ime(),
        detail: if probe.layout_is_ime() {
            "caller thread's keyboard layout is an IME (ImmIsIME)".to_string()
        } else {
            "caller thread's keyboard layout is not an IME — a composition \
             would be a silent no-op"
                .to_string()
        },
    });
    let (has_ctx, ctx_detail) = match probe.context(hwnd) {
        Some(ctx) => {
            probe.release_context(hwnd, ctx);
            (true, "ImmGetContext returned an input context".to_string())
        }
        None => (false, "ImmGetContext returned no input context".to_string()),
    };
    checks.push(PreflightCheck {
        family: "ime_context",
        effective: has_ctx,
        detail: ctx_detail,
    });
    checks.push(PreflightCheck {
        family: "ime_ui",
        effective: probe.default_ime_window(hwnd),
        detail: if probe.default_ime_window(hwnd) {
            "default IME window exists — IME enabled for this thread".to_string()
        } else {
            "no default IME window — IME not enabled for this thread".to_string()
        },
    });
    let effective = checks.iter().all(|check| check.effective);
    PreflightReport { effective, checks }
}

/// Folds a raw delivery log into the [`ImeDelivery`] flags — pure and
/// platform-independent so the verdict-relevant analysis is unit
/// tested everywhere. Consumed by the Windows delivery path and the
/// tests; gated out of non-Windows non-test builds (dead code).
#[cfg(any(windows, test))]
fn analyze(messages: Vec<ImeDeliveredMessage>, observed: bool) -> ImeDelivery {
    let mut delivery = ImeDelivery {
        observed,
        start_composition: messages
            .iter()
            .any(|msg| msg.message == WM_IME_STARTCOMPOSITION_ID),
        result_composition: messages.iter().any(|msg| {
            msg.message == WM_IME_COMPOSITION_ID && (msg.lparam & GCS_RESULTSTR_BIT) != 0
        }),
        end_composition: messages
            .iter()
            .any(|msg| msg.message == WM_IME_ENDCOMPOSITION_ID),
        committed_chars: messages
            .iter()
            .filter(|msg| matches!(msg.message, WM_IME_CHAR_ID | WM_CHAR_ID))
            .count(),
        ..ImeDelivery::default()
    };
    delivery.committed_text = {
        let text: String = messages
            .iter()
            .filter(|msg| matches!(msg.message, WM_IME_CHAR_ID | WM_CHAR_ID))
            .filter_map(|msg| msg.text)
            .collect();
        (!text.is_empty()).then_some(text)
    };
    delivery.result_text = messages
        .iter()
        .filter_map(|msg| msg.result_str.clone())
        .find(|text| !text.is_empty());
    delivery.composition_text = messages
        .iter()
        .filter_map(|msg| msg.comp_str.clone())
        .find(|text| !text.is_empty());
    delivery.messages = messages;
    delivery
}

/// Installs the lane tap on `hwnd`, pumps the caller thread's queue
/// for up to `budget`, restores the original `WndProc`, and returns
/// the analysed delivery. `limit` caps the retained log so a
/// pathological flood stays bounded.
///
/// The tap is a `GWLP_WNDPROC` subclass owned by the *calling
/// thread* — legal because the lane's editor child HWNDs are
/// caller-thread-owned (the D1 deviation). It delegates every message
/// to the previous `WndProc` after recording; the original proc is
/// restored before return on every path, and the tap function lives
/// in this binary so nothing can dangle after a DLL unload.
///
/// Returns `ImeDelivery{observed: false}` when the oracle machinery
/// cannot run (dead HWND / foreign-thread target / no backend) — the
/// caller must not treat that as "no messages were posted".
#[must_use]
pub fn collect_ime_messages(hwnd: u64, budget: Duration, limit: usize) -> ImeDelivery {
    imp::collect(hwnd, budget, limit)
}

/// The composed drive+oracle the lane consumes: installs the lane
/// tap on `hwnd`, runs [`drive_composition`] while it is live (the
/// `WM_IME_*` envelope is delivered synchronously into the target's
/// `WndProc` during the IMM32 calls — a post-drive tap would miss
/// it), drains the caller thread's queue for up to `budget`,
/// restores the original `WndProc`, and returns the drive outcome
/// plus the analysed [`ImeDelivery`].
///
/// When the tap cannot be installed the drive still runs — its own
/// outcome reports truthfully while the delivery reports
/// `observed: false` (accepted-but-unverifiable, never "nothing was
/// delivered").
#[must_use]
pub fn drive_composition_observed(
    hwnd: u64,
    text: &str,
    budget: Duration,
    limit: usize,
) -> (OsImeOutcome, ImeDelivery) {
    if !supported() {
        return (
            OsImeOutcome::Unsupported {
                reason: "OS-level IME composition drive is implemented for \
                     Windows (IMM32) only"
                    .to_string(),
            },
            ImeDelivery::default(),
        );
    }
    imp::observed_drive(hwnd, text, budget, limit)
}

#[cfg(windows)]
mod imp {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{
        GetLastError, HWND, LPARAM, LRESULT, SetLastError, WPARAM,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::Ime::{
        GCS_COMPSTR, GCS_RESULTSTR, ImmGetCompositionStringW, ImmGetContext, ImmGetDefaultIMEWnd,
        ImmGetOpenStatus, ImmIsIME, ImmNotifyIME, ImmReleaseContext, ImmSetCompositionStringW,
        ImmSetOpenStatus, NI_COMPOSITIONSTR, SCS_SETSTR,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, GetKeyboardLayout, SetFocus};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, DispatchMessageW, GWLP_WNDPROC, GetWindowThreadProcessId,
        IsWindow, MSG, PM_REMOVE, PeekMessageW, SetWindowLongPtrW, TranslateMessage, WNDPROC,
    };

    use super::{
        ImeDeliveredMessage, ImeDelivery, ImeProbe, OsImeOutcome, WM_CHAR_ID, WM_IME_CHAR_ID,
        WM_IME_COMPOSITION_ID, WM_IME_ENDCOMPOSITION_ID, WM_IME_KEYUP_ID, WM_IME_SETCONTEXT_ID,
        WM_IME_STARTCOMPOSITION_ID, analyze,
    };

    /// The real IMM32/Win32 probe.
    pub(super) struct OsIme;

    #[expect(unsafe_code)]
    impl ImeProbe for OsIme {
        fn current_thread(&self) -> u32 {
            // SAFETY: query-only thread-id read.
            unsafe { GetCurrentThreadId() }
        }

        fn owner_thread(&self, hwnd: u64) -> u32 {
            // SAFETY: query-only; accepts any HWND value.
            unsafe {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd as HWND, &raw mut pid)
            }
        }

        fn focused(&self, hwnd: u64) -> bool {
            // SAFETY: `SetFocus`/`GetFocus` on a caller-owned window.
            unsafe {
                SetFocus(hwnd as HWND);
                GetFocus() == hwnd as HWND
            }
        }

        fn context(&self, hwnd: u64) -> Option<u64> {
            // SAFETY: query-only; the caller releases via
            // `release_context` below.
            unsafe {
                let himc = ImmGetContext(hwnd as HWND);
                (!himc.is_null()).then_some(himc as usize as u64)
            }
        }

        fn release_context(&self, hwnd: u64, ctx: u64) {
            // SAFETY: `ctx` is a context `context` returned, released
            // exactly once by contract.
            unsafe {
                ImmReleaseContext(hwnd as HWND, ctx as _);
            }
        }

        fn open_status(&self, ctx: u64) -> bool {
            // SAFETY: query-only on a live context token.
            unsafe { ImmGetOpenStatus(ctx as _) != 0 }
        }

        fn set_open_status(&self, ctx: u64, open: bool) -> bool {
            // SAFETY: `ctx` is a live context token.
            unsafe { ImmSetOpenStatus(ctx as _, i32::from(open)) != 0 }
        }

        fn set_composition(&self, ctx: u64, text: &str) -> bool {
            // SAFETY: `wide` outlives the call; `lpComp`/`dwCompLen`
            // follow the `SCS_SETSTR|GCS_COMPSTR` contract (byte
            // length of a UTF-16 buffer).
            unsafe {
                let wide: Vec<u16> = text.encode_utf16().collect();
                let len = u32::try_from(wide.len() * 2).unwrap_or(u32::MAX);
                ImmSetCompositionStringW(
                    ctx as _,
                    SCS_SETSTR | GCS_COMPSTR,
                    wide.as_ptr().cast(),
                    len,
                    core::ptr::null(),
                    0,
                ) != 0
            }
        }

        fn notify(&self, ctx: u64, op: u32) -> bool {
            // SAFETY: `ctx` is a live context token.
            unsafe { ImmNotifyIME(ctx as _, NI_COMPOSITIONSTR, op, 0) != 0 }
        }

        fn layout_is_ime(&self) -> bool {
            // SAFETY: query-only layout/IME check.
            unsafe {
                let hkl = GetKeyboardLayout(0);
                !hkl.is_null() && ImmIsIME(hkl) != 0
            }
        }

        fn default_ime_window(&self, hwnd: u64) -> bool {
            // SAFETY: query-only.
            unsafe { !ImmGetDefaultIMEWnd(hwnd as HWND).is_null() }
        }
    }

    /// One tap's recorded state — a single shared slot is enough: the
    /// lane drives IME on one window at a time, and the subclass proc
    /// only ever runs on the owning (calling) thread.
    struct TapState {
        /// The previous `GWLP_WNDPROC` value — restored on tap drop.
        old_proc: isize,
        /// The recorded delivery log.
        log: Vec<ImeDeliveredMessage>,
    }

    static TAP: Mutex<TapState> = Mutex::new(TapState {
        old_proc: 0,
        log: Vec::new(),
    });

    /// Bound on the retained log — a pathological flood stays finite.
    const TAP_LOG_LIMIT: usize = 512;

    /// Messages worth recording — the `WM_IME_*` families plus
    /// `WM_CHAR` (`DefWindowProc`'s committed-text translation).
    fn loggable(message: u32) -> bool {
        message == WM_CHAR_ID
            || (WM_IME_STARTCOMPOSITION_ID..=WM_IME_COMPOSITION_ID).contains(&message)
            || (WM_IME_SETCONTEXT_ID..=WM_IME_KEYUP_ID).contains(&message)
    }

    /// `GCS_*` string read inside a `WM_IME_COMPOSITION` dispatch —
    /// the same `ImmGetCompositionStringW` sequence an IME-aware
    /// window performs. Query-only.
    #[expect(unsafe_code)]
    fn read_gcs(hwnd: HWND, index: u32) -> Option<String> {
        // SAFETY: query-only composition-string read on the window's
        // own input context, released before return.
        unsafe {
            let himc = ImmGetContext(hwnd);
            if himc.is_null() {
                return None;
            }
            let needed = ImmGetCompositionStringW(himc, index, core::ptr::null_mut(), 0);
            let text = if needed > 0 {
                let mut buf = vec![0u16; usize::try_from(needed).unwrap_or(0) / 2];
                let got = ImmGetCompositionStringW(
                    himc,
                    index,
                    buf.as_mut_ptr().cast(),
                    u32::try_from(needed).unwrap_or(0),
                );
                (got > 0).then(|| {
                    String::from_utf16_lossy(&buf[..usize::try_from(got).unwrap_or(0) / 2])
                })
            } else {
                None
            };
            ImmReleaseContext(hwnd, himc);
            text
        }
    }

    /// The tap `WndProc` — records `WM_IME_*`/`WM_CHAR` into the
    /// shared log, then delegates to the previous `WndProc`. The
    /// lock is never held across `CallWindowProcW`: the delegate may
    /// synchronously generate further deliveries into this proc
    /// (`DefWindowProc` emits `WM_IME_CHAR` while handling
    /// `WM_IME_COMPOSITION`), and a held non-reentrant lock would
    /// deadlock the re-entry.
    #[expect(unsafe_code)]
    unsafe extern "system" fn tap_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            let prev = {
                let mut guard = match TAP.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let prev = guard.old_proc;
                if loggable(msg) && guard.log.len() < TAP_LOG_LIMIT {
                    let (comp_str, result_str) = if msg == WM_IME_COMPOSITION_ID {
                        (read_gcs(hwnd, GCS_COMPSTR), read_gcs(hwnd, GCS_RESULTSTR))
                    } else {
                        (None, None)
                    };
                    let text = if matches!(msg, WM_IME_CHAR_ID | WM_CHAR_ID) {
                        u32::try_from(w).ok().and_then(char::from_u32)
                    } else {
                        None
                    };
                    guard.log.push(ImeDeliveredMessage {
                        message: msg,
                        wparam: w,
                        lparam: l,
                        text,
                        comp_str,
                        result_str,
                    });
                }
                prev
            };
            // SAFETY: `prev` is the WndProc value `SetWindowLongPtrW`
            // returned for this same HWND — a valid WNDPROC while the
            // window lives; `DefWindowProcW` is the defined fallback.
            match core::mem::transmute::<isize, WNDPROC>(prev) {
                Some(prev_fn) => CallWindowProcW(Some(prev_fn), hwnd, msg, w, l),
                None => DefWindowProcW(hwnd, msg, w, l),
            }
        }
    }

    /// The installed lane tap — owns the obligation to restore the
    /// original `WndProc` and take the recorded log via [`Tap::finish`].
    /// Install before driving, pump after: the `WM_IME_*` envelope is
    /// delivered synchronously into the `WndProc` during the IMM32
    /// calls, so the tap must already be live.
    struct Tap {
        target: HWND,
    }

    impl Tap {
        /// Installs the tap on a live caller-owned `hwnd` — `None`
        /// when the oracle machinery cannot run (dead HWND,
        /// foreign-thread target, refused `SetWindowLongPtrW`).
        #[expect(unsafe_code)]
        fn install(hwnd: u64) -> Option<Tap> {
            // SAFETY: `target` is only ever queried/messaged, never
            // destroyed, while the caller holds the lease.
            unsafe {
                let target = usize::try_from(hwnd).unwrap_or(0) as HWND;
                if IsWindow(target) == 0 {
                    return None;
                }
                let mut pid = 0u32;
                if GetWindowThreadProcessId(target, &raw mut pid) != GetCurrentThreadId() {
                    // Another thread's queue is not pumpable here.
                    return None;
                }
                // `SetWindowLongPtrW` returns the previous WndProc
                // (the restore value) with the `SetLastError`
                // disambiguation for a legitimately-0 one.
                let mut guard = match TAP.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.log.clear();
                SetLastError(0);
                guard.old_proc =
                    SetWindowLongPtrW(target, GWLP_WNDPROC, tap_proc as *const () as isize);
                if guard.old_proc == 0 && GetLastError() != 0 {
                    guard.old_proc = 0;
                    return None;
                }
                Some(Tap { target })
            }
        }

        /// Bounded pump: retrieve+dispatch every queued message for
        /// the target so the OS's posted `WM_IME_*` and any
        /// `DefWindowProc`-generated `WM_IME_CHAR`/`WM_CHAR` all flow
        /// through the tap. Ends when the composition terminator
        /// drained plus a short grace, the deadline passes, or the
        /// log cap is hit — never an unbounded wait.
        #[expect(unsafe_code)]
        fn pump(&self, budget: Duration, limit: usize) {
            // SAFETY: `self.target` is the tap-installed HWND; pump is
            // query-and-dispatch on the caller thread's own queue.
            unsafe {
                let deadline = Instant::now() + budget;
                let grace = Duration::from_millis(60);
                let mut end_seen_at: Option<Instant> = None;
                loop {
                    let mut msg = MSG::default();
                    if PeekMessageW(&raw mut msg, self.target, 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&raw const msg);
                        DispatchMessageW(&raw const msg);
                        if msg.message == WM_IME_ENDCOMPOSITION_ID {
                            end_seen_at = Some(Instant::now());
                        }
                        let logged = TAP.lock().map_or(0, |tap| tap.log.len());
                        if logged >= limit.min(TAP_LOG_LIMIT) {
                            break;
                        }
                        continue;
                    }
                    let now = Instant::now();
                    if end_seen_at.is_some_and(|seen| now >= seen + grace) || now >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }

        /// Restores the original `WndProc` (when the window still
        /// lives) and takes the recorded log — the tap never outlives
        /// the observation window.
        #[expect(unsafe_code)]
        fn finish(self) -> Vec<ImeDeliveredMessage> {
            // SAFETY: restore writes back the value `install`
            // received; `IsWindow` guards a concurrently-destroyed
            // HWND.
            unsafe {
                let mut guard = match TAP.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if IsWindow(self.target) != 0 {
                    SetWindowLongPtrW(self.target, GWLP_WNDPROC, guard.old_proc);
                }
                guard.old_proc = 0;
                std::mem::take(&mut guard.log)
            }
        }
    }

    /// Subclass + bounded pump + restore + analysis — see
    /// [`super::collect_ime_messages`]. Query-and-dispatch only:
    /// nothing is created, destroyed, or left subclassed.
    pub(super) fn collect(hwnd: u64, budget: Duration, limit: usize) -> ImeDelivery {
        let Some(tap) = Tap::install(hwnd) else {
            return ImeDelivery::default();
        };
        tap.pump(budget, limit);
        analyze(tap.finish(), true)
    }

    /// Tap-wrapped drive — see [`super::drive_composition_observed`].
    /// The tap is installed *before* `drive_inner` runs so the
    /// synchronously-delivered `WM_IME_*` envelope lands in the log;
    /// the queue drain then picks up whatever `DefWindowProc` posted.
    pub(super) fn observed_drive(
        hwnd: u64,
        text: &str,
        budget: Duration,
        limit: usize,
    ) -> (OsImeOutcome, ImeDelivery) {
        let Some(tap) = Tap::install(hwnd) else {
            // Oracle machinery cannot run — the drive still runs so
            // its own outcome reports truthfully (NotRun for a dead/
            // foreign HWND, EnvironmentBlocked, …) while the delivery
            // honestly reports `observed: false`.
            return (
                super::drive_inner(&OsIme, hwnd, text),
                ImeDelivery::default(),
            );
        };
        let outcome = super::drive_inner(&OsIme, hwnd, text);
        tap.pump(budget, limit);
        (outcome, analyze(tap.finish(), true))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::time::Duration;

    use super::{ImeDelivery, ImeProbe, OsImeOutcome};

    /// Non-Windows stub — `supported()` is false here so the public
    /// entry points never reach this probe's dishonest-looking zeros;
    /// it exists so the module compiles on every target.
    pub(super) struct OsIme;

    impl ImeProbe for OsIme {
        fn current_thread(&self) -> u32 {
            0
        }
        fn owner_thread(&self, _hwnd: u64) -> u32 {
            0
        }
        fn focused(&self, _hwnd: u64) -> bool {
            false
        }
        fn context(&self, _hwnd: u64) -> Option<u64> {
            None
        }
        fn release_context(&self, _hwnd: u64, _ctx: u64) {}
        fn open_status(&self, _ctx: u64) -> bool {
            false
        }
        fn set_open_status(&self, _ctx: u64, _open: bool) -> bool {
            false
        }
        fn set_composition(&self, _ctx: u64, _text: &str) -> bool {
            false
        }
        fn notify(&self, _ctx: u64, _op: u32) -> bool {
            false
        }
        fn layout_is_ime(&self) -> bool {
            false
        }
        fn default_ime_window(&self, _hwnd: u64) -> bool {
            false
        }
    }

    /// No IMM32 backend — the oracle honestly reports it could not
    /// run (`observed == false`), never an empty "no messages" claim.
    pub(super) fn collect(_hwnd: u64, _budget: Duration, _limit: usize) -> ImeDelivery {
        ImeDelivery::default()
    }

    /// No IMM32 backend — unreachable in practice (the public entry
    /// gates on `supported()`), honest `Unsupported` + unobserved
    /// delivery if ever reached.
    pub(super) fn observed_drive(
        _hwnd: u64,
        _text: &str,
        _budget: Duration,
        _limit: usize,
    ) -> (OsImeOutcome, ImeDelivery) {
        (
            OsImeOutcome::Unsupported {
                reason: "OS-level IME composition drive is implemented for \
                     Windows (IMM32) only"
                    .to_string(),
            },
            ImeDelivery::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// Scripted [`ImeProbe`] — records calls, plays back configured
    /// results so the drive's blocked/refused/precondition branches
    /// are verifiable without a real IME.
    #[derive(Default)]
    struct MockIme {
        owner: Cell<u32>,
        caller: Cell<u32>,
        focus_ok: Cell<bool>,
        ctx: Cell<Option<u64>>,
        open: Cell<bool>,
        set_ok: Cell<bool>,
        notify_ok: Cell<bool>,
        layout_ime: Cell<bool>,
        ime_wnd: Cell<bool>,
        calls: RefCell<Vec<&'static str>>,
    }

    impl MockIme {
        fn live() -> Self {
            Self {
                owner: Cell::new(7),
                caller: Cell::new(7),
                focus_ok: Cell::new(true),
                ctx: Cell::new(Some(0x00C0_FFEE)),
                open: Cell::new(true),
                set_ok: Cell::new(true),
                notify_ok: Cell::new(true),
                layout_ime: Cell::new(true),
                ime_wnd: Cell::new(true),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl ImeProbe for MockIme {
        fn current_thread(&self) -> u32 {
            self.caller.get()
        }
        fn owner_thread(&self, _hwnd: u64) -> u32 {
            self.owner.get()
        }
        fn focused(&self, _hwnd: u64) -> bool {
            self.calls.borrow_mut().push("focus");
            self.focus_ok.get()
        }
        fn context(&self, _hwnd: u64) -> Option<u64> {
            self.calls.borrow_mut().push("context");
            self.ctx.get()
        }
        fn release_context(&self, _hwnd: u64, _ctx: u64) {
            self.calls.borrow_mut().push("release");
        }
        fn open_status(&self, _ctx: u64) -> bool {
            self.open.get()
        }
        fn set_open_status(&self, _ctx: u64, open: bool) -> bool {
            self.calls.borrow_mut().push("set_open");
            self.open.set(open);
            true
        }
        fn set_composition(&self, _ctx: u64, _text: &str) -> bool {
            self.calls.borrow_mut().push("set_composition");
            self.set_ok.get()
        }
        fn notify(&self, _ctx: u64, op: u32) -> bool {
            self.calls.borrow_mut().push("notify");
            let _ = op;
            self.notify_ok.get()
        }
        fn layout_is_ime(&self) -> bool {
            self.layout_ime.get()
        }
        fn default_ime_window(&self, _hwnd: u64) -> bool {
            self.ime_wnd.get()
        }
    }

    #[test]
    fn drive_injected_when_every_call_is_accepted() {
        let probe = MockIme::live();
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(
            outcome,
            OsImeOutcome::Injected {
                action: "ime-compose-commit",
                ..
            }
        ));
        assert_eq!(
            *probe.calls.borrow(),
            vec!["focus", "context", "set_composition", "notify", "release"]
        );
    }

    #[test]
    fn drive_not_run_on_foreign_thread() {
        let probe = MockIme::live();
        probe.owner.set(9);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::NotRun { .. }));
        assert!(probe.calls.borrow().is_empty());
    }

    #[test]
    fn drive_not_run_on_dead_hwnd() {
        let probe = MockIme::live();
        probe.owner.set(0);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::NotRun { .. }));
    }

    #[test]
    fn drive_not_run_on_empty_text() {
        let probe = MockIme::live();
        let outcome = drive_inner(&probe, 0xBEEF, "");
        assert!(matches!(outcome, OsImeOutcome::NotRun { .. }));
    }

    #[test]
    fn drive_blocked_when_focus_refused() {
        let probe = MockIme::live();
        probe.focus_ok.set(false);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::EnvironmentBlocked { .. }));
        // Focus failure stops before any context work.
        assert_eq!(*probe.calls.borrow(), vec!["focus"]);
    }

    #[test]
    fn drive_blocked_when_no_context() {
        let probe = MockIme::live();
        probe.ctx.set(None);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::EnvironmentBlocked { .. }));
    }

    #[test]
    fn drive_blocked_when_composition_refused() {
        let probe = MockIme::live();
        probe.set_ok.set(false);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::EnvironmentBlocked { .. }));
        assert!(probe.calls.borrow().contains(&"release"));
    }

    #[test]
    fn drive_blocked_when_commit_refused_cancels() {
        let probe = MockIme::live();
        probe.notify_ok.set(false);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::EnvironmentBlocked { .. }));
        // CPS_CANCEL cleanup then release.
        assert_eq!(
            *probe.calls.borrow(),
            vec![
                "focus",
                "context",
                "set_composition",
                "notify",
                "notify",
                "release"
            ]
        );
    }

    #[test]
    fn drive_restores_open_status_when_it_opened_the_ime() {
        let probe = MockIme::live();
        probe.open.set(false);
        let outcome = drive_inner(&probe, 0xBEEF, "かな");
        assert!(matches!(outcome, OsImeOutcome::Injected { .. }));
        // open (true) then restore (false) — two set_open calls.
        assert_eq!(
            probe
                .calls
                .borrow()
                .iter()
                .filter(|c| **c == "set_open")
                .count(),
            2
        );
        assert!(!probe.open.get());
    }

    #[test]
    fn preflight_effective_only_when_all_present() {
        let probe = MockIme::live();
        assert!(preflight_inner(&probe, 0xBEEF).effective);
        probe.layout_ime.set(false);
        assert!(!preflight_inner(&probe, 0xBEEF).effective);
        probe.layout_ime.set(true);
        probe.ctx.set(None);
        let report = preflight_inner(&probe, 0xBEEF);
        assert!(!report.effective);
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.family == "ime_context" && !c.effective)
        );
        probe.ctx.set(Some(1));
        probe.ime_wnd.set(false);
        assert!(!preflight_inner(&probe, 0xBEEF).effective);
    }

    #[test]
    fn analyze_requires_full_commit_sequence() {
        let msg = |message: u32, lparam: isize| ImeDeliveredMessage {
            message,
            wparam: 0,
            lparam,
            text: None,
            comp_str: None,
            result_str: None,
        };
        // Only a start — not confirmed.
        let d = analyze(vec![msg(WM_IME_STARTCOMPOSITION_ID, 0)], true);
        assert!(!d.commit_confirmed());
        // Start + result-flagged composition + end, but no text.
        let d = analyze(
            vec![
                msg(WM_IME_STARTCOMPOSITION_ID, 0),
                msg(WM_IME_COMPOSITION_ID, GCS_RESULTSTR_BIT),
                msg(WM_IME_ENDCOMPOSITION_ID, 0),
            ],
            true,
        );
        assert!(!d.commit_confirmed());
        // With a committed char — confirmed.
        let mut char_msg = msg(WM_IME_CHAR_ID, 0);
        char_msg.wparam = u32::from('か') as usize;
        char_msg.text = Some('か');
        let d = analyze(
            vec![
                msg(WM_IME_STARTCOMPOSITION_ID, 0),
                msg(WM_IME_COMPOSITION_ID, GCS_RESULTSTR_BIT),
                char_msg,
                msg(WM_IME_ENDCOMPOSITION_ID, 0),
            ],
            true,
        );
        assert!(d.commit_confirmed());
        assert_eq!(d.committed_chars, 1);
        assert_eq!(d.committed_text.as_deref(), Some("か"));
        // A result-string read alone also proves committed text.
        let mut comp = msg(WM_IME_COMPOSITION_ID, GCS_RESULTSTR_BIT);
        comp.result_str = Some("かな".to_string());
        let d = analyze(
            vec![
                msg(WM_IME_STARTCOMPOSITION_ID, 0),
                comp,
                msg(WM_IME_ENDCOMPOSITION_ID, 0),
            ],
            true,
        );
        assert!(d.commit_confirmed());
        assert_eq!(d.result_text.as_deref(), Some("かな"));
        // Composition-flagged message without the result bit does not
        // count as a commit.
        let d = analyze(
            vec![
                msg(WM_IME_STARTCOMPOSITION_ID, 0),
                msg(WM_IME_COMPOSITION_ID, 0x0008),
                msg(WM_IME_ENDCOMPOSITION_ID, 0),
            ],
            true,
        );
        assert!(!d.result_composition);
        assert!(!d.commit_confirmed());
    }
}
