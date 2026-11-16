//! OS window-state mutation for the real-window dock lane
//! (Issue #532 foreground-probe slice — feature `real-dock`).
//!
//! This is the *only* module in the crate that mutates window
//! state. The sibling query modules (`osoccl`/`ospix`) keep their
//! read-only contract; everything here is confined to the GPUI
//! root HWND's z-order band and foreground ownership, applied and
//! restored inside one bounded probe case.
//!
//! Mutation families:
//!
//! - `raise_to_top` — `SetWindowPos(root, HWND_TOP,
//!   SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)`: moves the root to the
//!   top of the *non-topmost* z-order band without activating it.
//!   Pure z-order isolation — no foreground/activation change.
//! - `set_foreground` — `SetForegroundWindow(root)`: requests
//!   foreground + activation. The foreground lock may refuse this
//!   (`applied:false` + `win32_error`); that refusal is recorded
//!   honestly, never disguised as success.
//! - `set_topmost` — `SetWindowPos(root, HWND_TOPMOST, ...)`:
//!   moves the root into the topmost band, above every
//!   non-topmost window regardless of z-order.
//! - `restore` — returns the root to its baseline band
//!   (`HWND_NOTOPMOST`/`HWND_TOPMOST` per the snapshot's `topmost`
//!   flag), repositions it below its baseline above-neighbor when
//!   one existed, and attempts to hand foreground back to the
//!   baseline owner when that owner was not the root itself.
//!
//! Restoration contract: only the `WS_EX_TOPMOST` flag is
//! *verified* (`topmost_restored`); z-order and foreground are
//! best-effort — `zorder_before/after` and
//! `foreground_before/attempted/final` are recorded as observed
//! values so a reader sees exactly what could and could not be
//! restored. Windows does not guarantee exact z-order or
//! foreground restoration.
//!
//! All values crossing the module boundary are plain integers,
//! booleans, strings, and [`osoccl::ExstyleFacts`] — no Win32 type
//! reaches the caller (the same `u64`-narrowed-handle convention
//! as `oschild`/`ospix`/`osoccl`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

use crate::osoccl::ExstyleFacts;

/// `WS_EX_TOPMOST` extended-style bit — same stable Win32 ABI value
/// `osoccl` declares platform-neutrally.
const WS_EX_TOPMOST: u64 = 0x0000_0008;

/// Safety cap on the top-level z-order walk — a real desktop has
/// far fewer top-level windows; hitting it records `truncated`
/// rather than looping on a corrupted chain.
const ZWALK_CAP: u32 = 512;

/// The GPUI root HWND's mutable window state — everything the
/// probe needs to know before mutating and to compare after.
#[derive(Debug, Clone)]
pub(crate) struct RootSnapshot {
    /// `false` only on the non-Windows stub — callers gate on the
    /// attach observation's `supported` before probing, so this is
    /// a defensive flag, not a second oracle.
    pub supported: bool,
    /// `GetAncestor(child, GA_ROOT)` — the GPUI top-level window.
    pub root: Option<u64>,
    /// The root's raw `GWL_EXSTYLE` facts — `None` when the query
    /// fails.
    pub exstyle: Option<ExstyleFacts>,
    /// `WS_EX_TOPMOST` bit of [`Self::exstyle`] — the band the
    /// restore must return the root to. `false` when `exstyle` is
    /// `None` (unknown is treated as non-topmost for restore).
    pub topmost: bool,
    /// Number of top-level windows above the root in z-order
    /// (`GW_HWNDPREV` walk) — `None` when there is no root.
    pub above_count: Option<u32>,
    /// The top-level window directly above the root — the
    /// `SetWindowPos` insert-after target for approximate z-order
    /// restore. `None` when the root is already topmost-in-band or
    /// the walk failed.
    pub above_hwnd: Option<u64>,
    /// The top-level window directly below the root.
    pub below_hwnd: Option<u64>,
    /// `GetForegroundWindow` — the current foreground owner. `None`
    /// when there is no foreground window or the query is
    /// unsupported. The owner may be the root itself, another of
    /// our windows, or a foreign-process window.
    pub foreground: Option<u64>,
    /// `true` when the `GW_HWNDPREV` walk hit [`ZWALK_CAP`] before
    /// reaching the top — `above_count` is then a lower bound.
    pub truncated: bool,
}

/// The result of one mutation API call — what was invoked, whether
/// it applied, and the post-call snapshot.
#[derive(Debug, Clone)]
pub(crate) struct MutationOutcome {
    /// Recording name of the API sequence, e.g.
    /// `"SetWindowPos(HWND_TOP,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)"`.
    pub api: &'static str,
    /// The API's `BOOL` result — `true` applied, `false` refused
    /// or failed. Recorded verbatim; never upgraded.
    pub applied: bool,
    /// `GetLastError` when `applied` is `false` — `None` when the
    /// call succeeded or the error read itself was unavailable.
    pub win32_error: Option<u32>,
    /// Post-call [`RootSnapshot`] — exstyle/topmost/z-order/
    /// foreground re-read so the caller sees the state the
    /// mutation actually produced.
    pub after: RootSnapshot,
}

/// The result of [`restore`] — every restore op issued plus the
/// final observed state and the verification fields.
#[derive(Debug, Clone)]
pub(crate) struct RestoreOutcome {
    /// The restore ops in issue order (band fix → z-order
    /// reposition → foreground hand-back attempt).
    pub ops: Vec<MutationOutcome>,
    /// The root snapshot after all restore ops.
    pub final_state: RootSnapshot,
    /// `final_state.topmost == baseline.topmost` — the one restore
    /// property Windows lets us verify deterministically.
    pub topmost_restored: bool,
    /// `(above_count, above_hwnd)` before restore — recorded for
    /// the reader; exact equality is not guaranteed.
    pub zorder_before: (Option<u32>, Option<u64>),
    /// `(above_count, above_hwnd)` after restore — same caveat.
    pub zorder_after: (Option<u32>, Option<u64>),
    /// The baseline foreground owner recorded before any mutation.
    pub foreground_before: Option<u64>,
    /// Whether a `SetForegroundWindow(foreground_before)` attempt
    /// was issued (only when the baseline owner was a live window
    /// other than the root).
    pub foreground_attempted: bool,
    /// The foreground owner after restore — may legitimately differ
    /// from `foreground_before` (foreign-process foreground is not
    /// forcibly restorable); recorded, never faked.
    pub foreground_final: Option<u64>,
}

/// Snapshot the mutable state of the GPUI root that `child`
/// belongs to — resolves `GA_ROOT`, then reads exstyle, the
/// top-level z-order position, and the foreground owner. Query
/// only; never mutates.
pub(crate) fn observe(child: u64) -> RootSnapshot {
    platform::observe(child)
}

/// `SetWindowPos(root, HWND_TOP, 0,0,0,0,
/// SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)` — raise the root to the
/// top of the non-topmost band without activating it.
pub(crate) fn raise_to_top(root: u64) -> MutationOutcome {
    platform::raise_to_top(root)
}

/// `SetForegroundWindow(root)` — request foreground + activation.
/// May be refused by the foreground lock; `applied:false` +
/// `win32_error` records that honestly.
pub(crate) fn set_foreground(root: u64) -> MutationOutcome {
    platform::set_foreground(root)
}

/// `SetWindowPos(root, HWND_TOPMOST, 0,0,0,0,
/// SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)` — move the root into the
/// topmost band.
pub(crate) fn set_topmost(root: u64) -> MutationOutcome {
    platform::set_topmost(root)
}

/// Restore the root toward `baseline`: return it to the baseline
/// topmost band, reposition it below its baseline above-neighbor
/// when one existed, and attempt to hand foreground back to the
/// baseline owner when that owner was a live window other than the
/// root. Every op's result is recorded; the verification fields
/// report what could and could not be confirmed.
pub(crate) fn restore(baseline: &RootSnapshot) -> RestoreOutcome {
    platform::restore(baseline)
}

#[cfg(windows)]
#[expect(unsafe_code)]
mod platform {
    use windows_sys::Win32::Foundation::{GetLastError, HWND, SetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GW_HWNDNEXT, GW_HWNDPREV, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
        IsWindow, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SetForegroundWindow, SetWindowPos,
    };

    use super::{
        ExstyleFacts, MutationOutcome, RestoreOutcome, RootSnapshot, WS_EX_TOPMOST, ZWALK_CAP,
    };

    /// Flags shared by every z-order/band mutation — move, size,
    /// and activation are all suppressed so only the z-order
    /// position and topmost band change.
    const ZORDER_FLAGS: SET_WINDOW_POS_FLAGS = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;

    /// Child → `GA_ROOT` → full mutable-state snapshot. Every read
    /// degrades independently: a failed exstyle or z-walk leaves
    /// its field `None` rather than failing the whole snapshot.
    pub(super) fn observe(child: u64) -> RootSnapshot {
        // SAFETY: `GetAncestor`/`GetWindowLongPtrW`/`GetWindow`/
        // `GetForegroundWindow`/`IsWindow` are read-only queries;
        // no window is created, moved, styled, re-ordered, or
        // messaged.
        unsafe {
            let root =
                windows_sys::Win32::UI::WindowsAndMessaging::GetAncestor(child as HWND, GA_ROOT);
            let root_u64 = (!root.is_null()).then_some(root as usize as u64);
            let (exstyle, topmost) = root_u64.map_or((None, false), |r| match exstyle(r as HWND) {
                Ok(facts) => {
                    let top = facts.value & WS_EX_TOPMOST != 0;
                    (Some(facts), top)
                }
                Err(_) => (None, false),
            });
            let (above_count, above_hwnd, below_hwnd, truncated) =
                root_u64.map_or((None, None, None, false), z_position_u64);
            RootSnapshot {
                supported: true,
                root: root_u64,
                exstyle,
                topmost,
                above_count,
                above_hwnd,
                below_hwnd,
                foreground: foreground_owner(),
                truncated,
            }
        }
    }

    /// `SetWindowPos(root, HWND_TOP, ZORDER_FLAGS)` — top of the
    /// non-topmost band, no activation.
    pub(super) fn raise_to_top(root: u64) -> MutationOutcome {
        mutation(
            "SetWindowPos(HWND_TOP,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)",
            root,
            HWND_TOP,
        )
    }

    /// `SetForegroundWindow(root)` — foreground + activation
    /// request; foreground-lock refusal is recorded, not hidden.
    pub(super) fn set_foreground(root: u64) -> MutationOutcome {
        // SAFETY: `SetForegroundWindow` is the intentional,
        // bounded mutation this module exists for; `GetLastError`
        // disambiguates a refused call from a successful one.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetForegroundWindow(root as HWND) != 0;
            let err = if ok { None } else { Some(GetLastError()) };
            (ok, err)
        };
        MutationOutcome {
            api: "SetForegroundWindow",
            applied,
            win32_error: error,
            after: observe_root(root),
        }
    }

    /// `SetWindowPos(root, HWND_TOPMOST, ZORDER_FLAGS)` — topmost
    /// band, no activation.
    pub(super) fn set_topmost(root: u64) -> MutationOutcome {
        mutation(
            "SetWindowPos(HWND_TOPMOST,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)",
            root,
            HWND_TOPMOST,
        )
    }

    /// Restore toward `baseline`: band fix → above-neighbor
    /// reposition → foreground hand-back attempt. Every op is
    /// recorded; only `topmost_restored` is a verified property.
    pub(super) fn restore(baseline: &RootSnapshot) -> RestoreOutcome {
        let Some(root) = baseline.root else {
            return empty_restore(baseline);
        };
        let mut ops = Vec::new();

        // 1) Return the root to its baseline band.
        let band = if baseline.topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        ops.push(mutation(
            if baseline.topmost {
                "SetWindowPos(HWND_TOPMOST,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)"
            } else {
                "SetWindowPos(HWND_NOTOPMOST,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)"
            },
            root,
            band,
        ));

        // 2) Approximate z-order restore — place the root directly
        //    below its baseline above-neighbor (insert-after that
        //    neighbor). Skipped when there was no above-neighbor
        //    (root was already band-top) or it is no longer alive.
        if let Some(above) = baseline.above_hwnd {
            // SAFETY: `IsWindow` is a read-only liveness query on an
            // observed handle before it is reused as insert-after.
            let alive = unsafe { IsWindow(above as HWND) } != 0;
            if alive {
                ops.push(mutation(
                    "SetWindowPos(<baseline-above-neighbor>,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)",
                    root,
                    above as HWND,
                ));
            }
        }

        // 3) Foreground hand-back — only when the baseline owner was
        //    a live window other than the root itself. The result is
        //    recorded; foreign-process foreground is not forcibly
        //    restorable, so a refusal here is data, not a failure.
        //    `IsWindow` guards the same recycled-handle hazard the
        //    z-order op checks above (it cannot detect reuse, but it
        //    does skip a plainly dead owner).
        let foreground_attempted = match baseline.foreground {
            Some(owner) if owner != root => {
                // SAFETY: `IsWindow` is a read-only liveness query on
                // an observed handle before it is reused as the
                // foreground hand-back target.
                (unsafe { IsWindow(owner as HWND) }) != 0
            }
            _ => false,
        };
        if foreground_attempted && let Some(owner) = baseline.foreground {
            // SAFETY: same bounded foreground mutation as
            // `set_foreground`, issued for restoration; the
            // error is recorded like every other op.
            let (applied, error) = unsafe {
                SetLastError(0);
                let ok = SetForegroundWindow(owner as HWND) != 0;
                let err = if ok { None } else { Some(GetLastError()) };
                (ok, err)
            };
            ops.push(MutationOutcome {
                api: "SetForegroundWindow(<baseline-owner>)",
                applied,
                win32_error: error,
                after: observe_root(root),
            });
        }

        let final_state = observe_root(root);
        RestoreOutcome {
            // Fail closed on an unreadable exstyle: `topmost:false`
            // is also the fallback when the `GWL_EXSTYLE` read
            // failed, so require a successful read before claiming
            // the band was verified restored.
            topmost_restored: final_state.exstyle.is_some()
                && final_state.topmost == baseline.topmost,
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (final_state.above_count, final_state.above_hwnd),
            foreground_before: baseline.foreground,
            foreground_attempted,
            foreground_final: final_state.foreground,
            ops,
            final_state,
        }
    }

    /// Shared `SetWindowPos` mutation — records the API name, the
    /// `BOOL` result, `GetLastError` on failure, and re-observes the
    /// root so the caller sees the state actually produced.
    fn mutation(api: &'static str, root: u64, insert_after: HWND) -> MutationOutcome {
        // SAFETY: `SetWindowPos` is the intentional, bounded
        // mutation this module exists for; `ZORDER_FLAGS` suppresses
        // move/size/activation so only z-order/band changes.
        // `GetLastError` disambiguates a refused call.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetWindowPos(root as HWND, insert_after, 0, 0, 0, 0, ZORDER_FLAGS) != 0;
            let err = if ok { None } else { Some(GetLastError()) };
            (ok, err)
        };
        MutationOutcome {
            api,
            applied,
            win32_error: error,
            after: observe_root(root),
        }
    }

    /// Re-observe a known root HWND — the mutation helpers already
    /// hold the root, so they skip the `GA_ROOT` resolution that
    /// `observe(child)` performs for callers holding a child.
    fn observe_root(root: u64) -> RootSnapshot {
        // The helpers (`exstyle`/`z_position_u64`/`foreground_owner`)
        // are safe wrappers that scope their own `unsafe` FFI calls.
        let hwnd = root as HWND;
        let (exstyle, topmost) = match exstyle(hwnd) {
            Ok(facts) => {
                let top = facts.value & WS_EX_TOPMOST != 0;
                (Some(facts), top)
            }
            Err(_) => (None, false),
        };
        let (above_count, above_hwnd, below_hwnd, truncated) = z_position_u64(root);
        RootSnapshot {
            supported: true,
            root: Some(root),
            exstyle,
            topmost,
            above_count,
            above_hwnd,
            below_hwnd,
            foreground: foreground_owner(),
            truncated,
        }
    }

    /// `z_position` variant taking a `u64` root — shared by
    /// `observe` (which resolves `GA_ROOT` first) and
    /// `observe_root` (which already holds the root).
    fn z_position_u64(root: u64) -> (Option<u32>, Option<u64>, Option<u64>, bool) {
        // SAFETY: read-only `GW_HWNDPREV`/`GW_HWNDNEXT` walk on a
        // top-level window — `GetWindow` never mutates state.
        unsafe {
            let hwnd = root as HWND;
            let mut above = 0u32;
            let mut above_hwnd = None;
            let mut cursor =
                windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(hwnd, GW_HWNDPREV);
            let mut truncated = false;
            while !cursor.is_null() {
                if above == 0 {
                    above_hwnd = Some(cursor as usize as u64);
                }
                above += 1;
                if above >= ZWALK_CAP {
                    truncated = true;
                    break;
                }
                cursor =
                    windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(cursor, GW_HWNDPREV);
            }
            let below = windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(hwnd, GW_HWNDNEXT);
            (
                Some(above),
                above_hwnd,
                (!below.is_null()).then_some(below as usize as u64),
                truncated,
            )
        }
    }

    /// `GWL_EXSTYLE` of `hwnd` — same raw-value convention as
    /// `osoccl`: `SetLastError(0)` disambiguates a legitimate `0`
    /// style from a failed call.
    fn exstyle(hwnd: HWND) -> Result<ExstyleFacts, String> {
        // SAFETY: read-only style query on an observed-live handle.
        unsafe {
            SetLastError(0);
            let raw =
                windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let error = GetLastError();
            if error != 0 {
                return Err(format!(
                    "GetWindowLongPtrW(GWL_EXSTYLE) failed (win32 error {error})"
                ));
            }
            let style =
                u32::try_from(raw).map_err(|_| "GWL_EXSTYLE value does not fit u32".to_string())?;
            Ok(ExstyleFacts {
                value: u64::from(style),
            })
        }
    }

    /// `GetForegroundWindow` as `u64` — `None` on NULL or an
    /// unsupported read.
    fn foreground_owner() -> Option<u64> {
        // SAFETY: read-only foreground query.
        let hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        (!hwnd.is_null()).then_some(hwnd as usize as u64)
    }

    /// Restore outcome for a baseline with no resolvable root —
    /// nothing was mutated, so there is nothing to restore; the
    /// fields still report the baseline honestly.
    fn empty_restore(baseline: &RootSnapshot) -> RestoreOutcome {
        RestoreOutcome {
            ops: Vec::new(),
            final_state: baseline.clone(),
            topmost_restored: true,
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (baseline.above_count, baseline.above_hwnd),
            foreground_before: baseline.foreground,
            foreground_attempted: false,
            foreground_final: baseline.foreground,
        }
    }
}

/// Non-Windows stub — `NativePluginDock` (and thus the case that
/// calls this) is Windows-only, so this path is unreachable in
/// practice; it exists so the crate compiles on every target and
/// records `supported: false` with empty fields, never fabricated
/// mutations or observations.
#[cfg(not(windows))]
mod platform {
    use super::{MutationOutcome, RestoreOutcome, RootSnapshot};

    fn unsupported_snapshot() -> RootSnapshot {
        RootSnapshot {
            supported: false,
            root: None,
            exstyle: None,
            topmost: false,
            above_count: None,
            above_hwnd: None,
            below_hwnd: None,
            foreground: None,
            truncated: false,
        }
    }

    fn unsupported_outcome(api: &'static str) -> MutationOutcome {
        MutationOutcome {
            api,
            applied: false,
            win32_error: None,
            after: unsupported_snapshot(),
        }
    }

    pub(super) fn observe(_child: u64) -> RootSnapshot {
        unsupported_snapshot()
    }

    pub(super) fn raise_to_top(_root: u64) -> MutationOutcome {
        unsupported_outcome("SetWindowPos(HWND_TOP,...)")
    }

    pub(super) fn set_foreground(_root: u64) -> MutationOutcome {
        unsupported_outcome("SetForegroundWindow")
    }

    pub(super) fn set_topmost(_root: u64) -> MutationOutcome {
        unsupported_outcome("SetWindowPos(HWND_TOPMOST,...)")
    }

    pub(super) fn restore(baseline: &RootSnapshot) -> RestoreOutcome {
        RestoreOutcome {
            ops: Vec::new(),
            final_state: baseline.clone(),
            topmost_restored: true,
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (baseline.above_count, baseline.above_hwnd),
            foreground_before: baseline.foreground,
            foreground_attempted: false,
            foreground_final: baseline.foreground,
        }
    }
}
