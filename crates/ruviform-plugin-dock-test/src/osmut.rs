//! OS window-state mutation for the real-window dock lane
//! (Issue #532 foreground-probe slice — feature `real-dock`).
//!
//! This is the *only* module in the crate that mutates window
//! state. The sibling query modules (`osoccl`/`ospix`) keep their
//! read-only contract; everything here is confined to the GPUI
//! root HWND's z-order band and foreground ownership plus the
//! leased container HWND's presentation relationship, applied and
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
//! - `repaint_container` — `RedrawWindow(container, NULL, NULL,
//!   RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)`: a synchronous forced
//!   repaint that clears any persisted update region before
//!   observation (Issue #532 band-escape probe).
//! - `apply_layered` — `GWL_EXSTYLE |= WS_EX_LAYERED` on the
//!   container + `SWP_FRAMECHANGED` style apply +
//!   `SetLayeredWindowAttributes(container, 0, 255, LWA_ALPHA)`:
//!   re-routes the container through a layered presentation
//!   surface instead of its inherited compositing path.
//! - `reparent_to_desktop` — restores the container's baseline
//!   exstyle, then `SetParent(container, GetDesktopWindow())` +
//!   `SetWindowPos` back onto its baseline screen rect at
//!   `HWND_TOP`: makes the container a top-level window so its
//!   composition no longer rides the GPUI root's presentation
//!   plane.
//! - `restore_container` — `SetParent` back to the baseline
//!   parent, baseline `GWL_EXSTYLE` + `SWP_FRAMECHANGED`, the
//!   baseline rect in parent-client coordinates at `HWND_TOP`,
//!   and a best-effort sibling z-order reposition.
//! - `drain_paint` — `UpdateWindow(hwnd)` plus a bounded
//!   caller-thread `PeekMessage`/`DispatchMessage` `WM_PAINT`
//!   pass: the (a2) update/pump experiment for the docked child
//!   HWND, which is owned by this thread, so its queued paint
//!   messages sit in this thread's queue and a bounded pass
//!   drains them deterministically — never an open-ended
//!   message loop.
//!
//! Restoration contract: only the `WS_EX_TOPMOST` flag is
//! *verified* on the root (`topmost_restored`); z-order and
//! foreground are best-effort — `zorder_before/after` and
//! `foreground_before/attempted/final` are recorded as observed
//! values so a reader sees exactly what could and could not be
//! restored. The container restore verifies `parent`, the raw
//! `GWL_EXSTYLE` value, and the screen rect deterministically
//! (`parent_restored`/`exstyle_restored`/`rect_restored`); its
//! sibling z-order is likewise best-effort only. Windows does
//! not guarantee exact z-order or foreground restoration.
//!
//! All values crossing the module boundary are plain integers,
//! booleans, strings, [`osoccl::ExstyleFacts`], and
//! [`ospix::ScreenRect`] — no Win32 type reaches the caller (the
//! same `u64`-narrowed-handle convention as
//! `oschild`/`ospix`/`osoccl`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

use crate::osoccl::ExstyleFacts;
use crate::ospix::ScreenRect;

/// `WS_EX_TOPMOST` extended-style bit — same stable Win32 ABI value
/// `osoccl` declares platform-neutrally.
const WS_EX_TOPMOST: u64 = 0x0000_0008;

/// `WS_EX_LAYERED` extended-style bit — the layered-surface flag the
/// band-escape probe ORs into the container's `GWL_EXSTYLE`. Same
/// stable Win32 ABI value `osoccl` decodes.
const WS_EX_LAYERED: u64 = 0x0008_0000;

/// Safety cap on the top-level z-order walk — a real desktop has
/// far fewer top-level windows; hitting it records `truncated`
/// rather than looping on a corrupted chain.
const ZWALK_CAP: u32 = 512;

/// Safety cap on the caller-thread paint pump — one `WM_PAINT`
/// dispatch is normally enough; the cap bounds the pass so a
/// wndproc that re-invalidates itself cannot turn the drain into
/// an unbounded message loop.
const PAINT_PUMP_CAP: u32 = 8;

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

/// The leased plugin-dock container HWND's mutable window state —
/// the snapshot the band-escape probe takes before mutating the
/// container's presentation relationship and compares after
/// restoring.
#[derive(Debug, Clone)]
pub(crate) struct ContainerSnapshot {
    /// `false` only on the non-Windows stub — callers gate on the
    /// attach observation's `supported` before probing, so this is
    /// a defensive flag, not a second oracle.
    pub supported: bool,
    /// The queried container HWND echoed back (the input handle —
    /// `None` only on the non-Windows stub, which takes no HWND).
    pub container: Option<u64>,
    /// `IsWindow(container)` — a dead container cannot be repainted,
    /// layered, reparented, or restored; every downstream op then
    /// fails honestly rather than mutating a recycled handle.
    pub alive: bool,
    /// `GetParent(container)` — the window the restore must return
    /// the container beneath. `None` when dead or unparented.
    pub parent: Option<u64>,
    /// The container's raw `GWL_EXSTYLE` facts — `None` when the
    /// query fails. The restore writes this value back verbatim.
    pub exstyle: Option<ExstyleFacts>,
    /// `GetClassLongPtrW(container, GCL_STYLE)` — the container's
    /// class-style bits. `CS_OWNDC`/`CS_CLASSDC` forbid
    /// `WS_EX_LAYERED`, so the band-escape evidence needs them to
    /// distinguish a refused layered write from a silently dropped
    /// one. `None` when dead or the read fails.
    pub class_style: Option<u64>,
    /// `GetWindowRect` in virtual-screen coordinates — the rect the
    /// reparent stage re-applies and the restore returns to (in
    /// parent-client coordinates). `None` when the query fails.
    pub rect: Option<ScreenRect>,
    /// Sibling windows above the container in its parent's z-order
    /// (`GW_HWNDPREV` walk) — `None` when the container is dead.
    pub above_count: Option<u32>,
    /// The sibling directly above the container — the
    /// `SetWindowPos` insert-after target for approximate z-order
    /// restore.
    pub above_hwnd: Option<u64>,
    /// The sibling directly below the container.
    pub below_hwnd: Option<u64>,
    /// `true` when the `GW_HWNDPREV` walk hit [`ZWALK_CAP`] before
    /// reaching the top — `above_count` is then a lower bound.
    pub truncated: bool,
}

/// The result of one container-mutation API call — what was
/// invoked, whether it applied, and the post-call container
/// snapshot.
#[derive(Debug, Clone)]
pub(crate) struct ContainerMutationOutcome {
    /// Recording name of the API call, e.g.
    /// `"SetParent(GetDesktopWindow())"`.
    pub api: &'static str,
    /// The API's success result — `true` applied, `false` refused
    /// or failed. Recorded verbatim; never upgraded.
    pub applied: bool,
    /// `GetLastError` when `applied` is `false` — `None` when the
    /// call succeeded or the error read itself was unavailable.
    pub win32_error: Option<u32>,
    /// Post-call [`ContainerSnapshot`] — parent/exstyle/rect/
    /// z-order re-read so the caller sees the state the mutation
    /// actually produced.
    pub after: ContainerSnapshot,
}

/// The result of [`restore_container`] — every restore op issued
/// plus the final observed state and the verification fields.
#[derive(Debug, Clone)]
pub(crate) struct ContainerRestoreOutcome {
    /// The restore ops in issue order (`SetParent` back → baseline
    /// `GWL_EXSTYLE` write → `SWP_FRAMECHANGED` apply → baseline
    /// rect re-seat → optional sibling insert-after reposition).
    pub ops: Vec<ContainerMutationOutcome>,
    /// The container snapshot after all restore ops.
    pub final_state: ContainerSnapshot,
    /// `final_state.parent == baseline.parent` — verified
    /// deterministically; `false` when either side is unknown.
    pub parent_restored: bool,
    /// `final_state.exstyle.value == baseline.exstyle.value` —
    /// verified deterministically; `false` when either read failed.
    pub exstyle_restored: bool,
    /// `final_state.rect == baseline.rect` — verified
    /// deterministically; `false` when either read failed.
    pub rect_restored: bool,
    /// `(above_count, above_hwnd)` before restore — recorded for
    /// the reader; exact equality is not guaranteed.
    pub zorder_before: (Option<u32>, Option<u64>),
    /// `(above_count, above_hwnd)` after restore — same caveat.
    pub zorder_after: (Option<u32>, Option<u64>),
}

/// The result of [`drain_paint`] — the bounded paint-drain ops
/// plus the update-region state read before and after.
#[derive(Debug)]
pub(crate) struct PaintDrainOutcome {
    /// `false` only on the non-Windows stub — callers gate on the
    /// attach observation's `supported` before probing, so this is
    /// a defensive flag, not a second oracle.
    pub supported: bool,
    /// The drained HWND echoed back.
    pub hwnd: u64,
    /// `GetUpdateRect` non-empty before the drain — the `ospres`
    /// `has_update` proxy re-read here so the probe records one
    /// source of truth. `None` when the read was unavailable.
    pub update_before: Option<bool>,
    /// The drain ops in issue order (`UpdateWindow`, then the
    /// bounded `PeekMessage`/`DispatchMessage` pump row).
    pub ops: Vec<ContainerMutationOutcome>,
    /// `WM_PAINT` messages the pump dispatched to the wndproc.
    pub dispatched: u32,
    /// `has_update` re-read after the drain.
    pub update_after: Option<bool>,
    /// `update_after == Some(false)` — the pending update region
    /// really cleared, the discriminating fact of the (a2)
    /// hypothesis.
    pub cleared: bool,
}

/// `UpdateWindow(hwnd)` plus a bounded caller-thread
/// `PeekMessage(WM_PAINT)`/`DispatchMessage` pass — the Issue #532
/// (a2) experiment: if the docked child HWND's pending update
/// region exists only because its owning thread (this caller
/// thread) never pumps `WM_PAINT`, draining it here must clear the
/// region and produce a real backing raster. `UpdateWindow` sends
/// `WM_PAINT` synchronously when the region is non-empty; the pump
/// then dispatches any `WM_PAINT` still queued for `hwnd` —
/// possible precisely because the child is owned by this thread —
/// bounded by [`PAINT_PUMP_CAP`]. Nothing here is a restorable
/// state mutation; the outcome records what ran and what the
/// update region reported.
pub(crate) fn drain_paint(hwnd: u64) -> PaintDrainOutcome {
    platform::drain_paint(hwnd)
}

/// Snapshot the mutable state of the leased container HWND —
/// parent, exstyle, screen rect, and sibling z-order position.
/// Query only; never mutates.
pub(crate) fn observe_container(container: u64) -> ContainerSnapshot {
    platform::observe_container(container)
}

/// `RedrawWindow(container, NULL, NULL,
/// RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)` — a synchronous forced
/// repaint of the whole container so a persisted update region is
/// drained before observation (`RDW_UPDATENOW` pumps `WM_PAINT`
/// inline; no settle needed).
pub(crate) fn repaint_container(container: u64) -> ContainerMutationOutcome {
    platform::repaint_container(container)
}

/// Layered-surface stage: `GWL_EXSTYLE |= WS_EX_LAYERED` on the
/// container, `SWP_FRAMECHANGED` so the new style takes effect, then
/// `SetLayeredWindowAttributes(container, 0, 255, LWA_ALPHA)` —
/// opaque alpha, so the window should look identical while its
/// presentation now rides a layered surface. The three ops return in
/// issue order; the caller records each and follows with
/// [`repaint_container`].
pub(crate) fn apply_layered(container: u64) -> Vec<ContainerMutationOutcome> {
    platform::apply_layered(container)
}

/// Desktop-reparent stage: write `baseline_exstyle` back first so
/// the escape runs on the pristine style, apply it with
/// `SWP_FRAMECHANGED`, then `SetParent(container,
/// GetDesktopWindow())` and `SetWindowPos` the window back onto
/// `rect` (screen coordinates — a top-level window's frame rect) at
/// `HWND_TOP`. The four ops return in issue order.
pub(crate) fn reparent_to_desktop(
    container: u64,
    baseline_exstyle: u64,
    rect: ScreenRect,
) -> Vec<ContainerMutationOutcome> {
    platform::reparent_to_desktop(container, baseline_exstyle, rect)
}

/// Restore the container toward `baseline`: `SetParent` back to the
/// baseline parent, write the baseline `GWL_EXSTYLE` back and apply
/// it with `SWP_FRAMECHANGED`, re-seat the baseline rect (converted
/// to parent-client coordinates) at `HWND_TOP`, then place the
/// container directly below its baseline above-neighbor when one
/// existed. Every op's result is recorded; the verification fields
/// report what could and could not be confirmed.
pub(crate) fn restore_container(baseline: &ContainerSnapshot) -> ContainerRestoreOutcome {
    platform::restore_container(baseline)
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
    use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, SetLastError};
    use windows_sys::Win32::Graphics::Gdi::{
        RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, ScreenToClient, UpdateWindow,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GA_ROOT, GCL_STYLE, GW_HWNDNEXT, GW_HWNDPREV, GWL_EXSTYLE,
        GetClassLongPtrW, GetDesktopWindow, GetParent, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
        IsWindow, LWA_ALPHA, MSG, PM_REMOVE, PeekMessageW, SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow,
        SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW, SetWindowPos, WM_PAINT,
    };

    use super::{
        ContainerMutationOutcome, ContainerRestoreOutcome, ContainerSnapshot, ExstyleFacts,
        MutationOutcome, PAINT_PUMP_CAP, PaintDrainOutcome, RestoreOutcome, RootSnapshot,
        ScreenRect, WS_EX_LAYERED, WS_EX_TOPMOST, ZWALK_CAP,
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

    /// `GCL_STYLE` of `hwnd`'s window class — raw bits as `u64`.
    /// `CS_OWNDC`/`CS_CLASSDC` forbid `WS_EX_LAYERED`, so the
    /// band-escape evidence needs the class style to distinguish a
    /// silently dropped `GWL_EXSTYLE` write from a refused one.
    /// `SetLastError(0)` disambiguates a legitimate `0` class style
    /// from a failed call.
    fn class_style_bits(hwnd: HWND) -> Result<u64, String> {
        // SAFETY: read-only class-style query on an observed-live
        // handle; `GetLastError` disambiguates a failed call.
        unsafe {
            SetLastError(0);
            let raw = GetClassLongPtrW(hwnd, GCL_STYLE);
            let error = GetLastError();
            if raw == 0 && error != 0 {
                return Err(format!(
                    "GetClassLongPtrW(GCL_STYLE) failed (win32 error {error})"
                ));
            }
            Ok(raw as u64)
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

    /// Full container mutable-state snapshot — liveness gates every
    /// read so a dead handle reports `alive:false` with every field
    /// `None` rather than probing a recycled HWND.
    pub(super) fn observe_container(container: u64) -> ContainerSnapshot {
        // SAFETY: `IsWindow`/`GetParent`/`GetWindow` are read-only
        // queries on an opaque handle; `window_screen_rect` and
        // `exstyle` scope their own read-only calls. No window is
        // created, moved, styled, re-parented, or messaged.
        let (alive, parent, facts, class_style, rect, zorder) = unsafe {
            let hwnd = container as HWND;
            let alive = IsWindow(hwnd) != 0;
            let parent = if alive {
                let parent = GetParent(hwnd);
                (!parent.is_null()).then_some(parent as usize as u64)
            } else {
                None
            };
            let facts = alive.then(|| exstyle(hwnd).ok()).flatten();
            let class_style = alive.then(|| class_style_bits(hwnd).ok()).flatten();
            let rect = alive
                .then(|| crate::ospix::window_screen_rect(container).ok())
                .flatten();
            let zorder = if alive {
                z_position_u64(container)
            } else {
                (None, None, None, false)
            };
            (alive, parent, facts, class_style, rect, zorder)
        };
        let (above_count, above_hwnd, below_hwnd, truncated) = zorder;
        ContainerSnapshot {
            supported: true,
            container: Some(container),
            alive,
            parent,
            exstyle: facts,
            class_style,
            rect,
            above_count,
            above_hwnd,
            below_hwnd,
            truncated,
        }
    }

    /// `GetUpdateRect` non-empty on `hwnd` — re-read through
    /// `ospres` so the probe records one source of truth for the
    /// pending-update fact.
    fn update_pending(hwnd: u64) -> Option<bool> {
        crate::ospres::attributes(hwnd)
            .update
            .as_ref()
            .map(|u| u.has_update)
    }

    /// `UpdateWindow(hwnd)` + bounded `PeekMessage(WM_PAINT)`/
    /// `DispatchMessage` pass — see [`super::drain_paint`]. The
    /// pump targets `hwnd` on this thread's own queue; the child
    /// HWND is owned by this thread, so each dispatch invokes its
    /// wndproc synchronously (`BeginPaint`/`EndPaint` clear the
    /// update region and produce the raster).
    pub(super) fn drain_paint(hwnd: u64) -> PaintDrainOutcome {
        let update_before = update_pending(hwnd);
        let mut ops = Vec::with_capacity(2);
        // SAFETY: `UpdateWindow` is the intentional synchronous-paint
        // call this arm exists for; `GetLastError` disambiguates a
        // failed call.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = UpdateWindow(hwnd as HWND) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ops.push(ContainerMutationOutcome {
            api: "UpdateWindow()",
            applied,
            win32_error: error,
            after: observe_container(hwnd),
        });
        // Bounded caller-thread pump — `PeekMessage` with the
        // hwnd + `WM_PAINT` filter synthesizes the pending paint
        // message for this thread's queue; `DispatchMessage`
        // invokes the wndproc on this thread. The pass stops at the
        // first empty read or `PAINT_PUMP_CAP` — a wndproc that
        // re-invalidates itself cannot keep it alive.
        let mut dispatched = 0u32;
        for _ in 0..PAINT_PUMP_CAP {
            // SAFETY: `msg` is a valid out/in pointer across both
            // calls; the message came from this thread's own queue,
            // so dispatch is synchronous on this thread.
            let found = unsafe {
                let mut msg = MSG::default();
                if PeekMessageW(&raw mut msg, hwnd as HWND, WM_PAINT, WM_PAINT, PM_REMOVE) == 0 {
                    false
                } else {
                    DispatchMessageW(&raw const msg);
                    true
                }
            };
            if !found {
                break;
            }
            dispatched += 1;
        }
        ops.push(ContainerMutationOutcome {
            api: "PeekMessageW(hwnd,WM_PAINT,PM_REMOVE)+DispatchMessageW[cap=8]",
            applied: true,
            win32_error: None,
            after: observe_container(hwnd),
        });
        let update_after = update_pending(hwnd);
        PaintDrainOutcome {
            supported: true,
            hwnd,
            update_before,
            ops,
            dispatched,
            update_after,
            cleared: update_after == Some(false),
        }
    }

    /// `RedrawWindow(container, NULL, NULL,
    /// RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)` — synchronous whole-
    /// window repaint; `RDW_UPDATENOW` drains the update region
    /// inline so the post-call observation sees a painted frame.
    pub(super) fn repaint_container(container: u64) -> ContainerMutationOutcome {
        // SAFETY: `RedrawWindow` is the intentional, bounded repaint
        // mutation this probe stage exists for; `GetLastError`
        // disambiguates a failed call.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = RedrawWindow(
                container as HWND,
                std::ptr::null(),
                std::ptr::null_mut(),
                RDW_INVALIDATE | RDW_ERASE | RDW_UPDATENOW,
            ) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ContainerMutationOutcome {
            api: "RedrawWindow(RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)",
            applied,
            win32_error: error,
            after: observe_container(container),
        }
    }

    /// The three layered-surface ops in issue order — exstyle OR,
    /// frame apply, `LWA_ALPHA` attributes.
    pub(super) fn apply_layered(container: u64) -> Vec<ContainerMutationOutcome> {
        let mut ops = Vec::with_capacity(3);
        let current = observe_container(container).exstyle.map(|e| e.value);
        ops.push(match current {
            Some(value) => exstyle_write(
                container,
                value | WS_EX_LAYERED,
                "SetWindowLongPtrW(GWL_EXSTYLE,<current>|WS_EX_LAYERED)",
            ),
            None => ContainerMutationOutcome {
                api: "SetWindowLongPtrW(GWL_EXSTYLE,<current>|WS_EX_LAYERED)",
                applied: false,
                win32_error: None,
                after: observe_container(container),
            },
        });
        ops.push(frame_apply(container));
        // SAFETY: `SetLayeredWindowAttributes` is the intentional,
        // bounded attribute mutation this probe stage exists for —
        // alpha 255 (fully opaque), no color key. `GetLastError`
        // disambiguates a failed call.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetLayeredWindowAttributes(container as HWND, 0, 255, LWA_ALPHA) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ops.push(ContainerMutationOutcome {
            api: "SetLayeredWindowAttributes(0,255,LWA_ALPHA)",
            applied,
            win32_error: error,
            after: observe_container(container),
        });
        ops
    }

    /// The four desktop-reparent ops in issue order — baseline
    /// exstyle restore, frame apply, `SetParent` to the desktop,
    /// rect re-seat at `HWND_TOP`.
    pub(super) fn reparent_to_desktop(
        container: u64,
        baseline_exstyle: u64,
        rect: ScreenRect,
    ) -> Vec<ContainerMutationOutcome> {
        let mut ops = Vec::with_capacity(4);
        ops.push(exstyle_write(
            container,
            baseline_exstyle,
            "SetWindowLongPtrW(GWL_EXSTYLE,<baseline>)",
        ));
        ops.push(frame_apply(container));
        // SAFETY: `SetParent` to `GetDesktopWindow` is the
        // intentional, bounded reparent this probe stage exists
        // for. A NULL return is a failed reparent; `GetLastError`
        // disambiguates.
        let (applied, error) = unsafe {
            SetLastError(0);
            let previous = SetParent(container as HWND, GetDesktopWindow());
            let ok = !previous.is_null();
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ops.push(ContainerMutationOutcome {
            api: "SetParent(GetDesktopWindow())",
            applied,
            win32_error: error,
            after: observe_container(container),
        });
        // The container is top-level now — its frame rect lives in
        // screen coordinates, so the baseline screen rect applies
        // verbatim.
        ops.push(rect_seat(
            container,
            HWND_TOP,
            rect.left,
            rect.top,
            rect.width,
            rect.height,
            "SetWindowPos(HWND_TOP,<baseline-screen-rect>,SWP_NOACTIVATE)",
        ));
        ops
    }

    /// Restore toward `baseline` — baseline exstyle + frame apply
    /// while still under the current parent, `SetParent` back,
    /// baseline rect in parent-client coordinates, then a
    /// best-effort insert-after reposition.
    /// Parent/exstyle/rect are verified deterministically; the
    /// sibling z-order is recorded, never claimed.
    pub(super) fn restore_container(baseline: &ContainerSnapshot) -> ContainerRestoreOutcome {
        let Some(container) = baseline.container else {
            return empty_container_restore(baseline);
        };
        let mut ops = Vec::new();

        // 1) Z-band restore toward the baseline, while the window
        //    is still under its current (possibly desktop) parent.
        //    `WS_EX_TOPMOST` is band state — a `GWL_EXSTYLE` write
        //    of the baseline value cannot clear it (observed: the
        //    `0x8 → 0` write returns without error yet the bit
        //    stays set), only `SetWindowPos(HWND_NOTOPMOST)` moves
        //    the window out of the topmost band. The op is issued
        //    before re-docking because band state is meaningless —
        //    and the write unreliable — under a foreign-thread
        //    parent (see the exstyle note below).
        if let Some(op) = band_restore_op(container, baseline) {
            ops.push(op);
        }

        // 2) Baseline exstyle + frame apply, still under the
        //    current parent. Evidence from the rf532 probe:
        //    `GWL_EXSTYLE` writes on the editor child are refused
        //    with win32 error 6 whenever its parent is owned by
        //    another thread — the container is GPUI-thread-owned —
        //    but succeed while the window is desktop-parented.
        //    Restoring the style bits (e.g. clearing
        //    `WS_EX_LAYERED` left by a treatment arm) is only
        //    reliable before the window is re-docked beneath the
        //    foreign-thread parent.
        if let Some(facts) = &baseline.exstyle {
            ops.push(exstyle_write(
                container,
                facts.value,
                "SetWindowLongPtrW(GWL_EXSTYLE,<baseline>)",
            ));
            ops.push(frame_apply(container));
        }

        // 3) Return the container beneath its baseline parent.
        if let Some(parent) = baseline.parent {
            // SAFETY: `SetParent` back to the observed baseline
            // parent is the intentional restore mutation;
            // `IsWindow` guards a plainly dead parent first (it
            // cannot detect handle reuse, but skips a dead one).
            let (applied, error) = unsafe {
                if IsWindow(parent as HWND) == 0 {
                    (false, None)
                } else {
                    SetLastError(0);
                    let previous = SetParent(container as HWND, parent as HWND);
                    let ok = !previous.is_null();
                    let error = if ok { None } else { Some(GetLastError()) };
                    (ok, error)
                }
            };
            ops.push(ContainerMutationOutcome {
                api: "SetParent(<baseline-parent>)",
                applied,
                win32_error: error,
                after: observe_container(container),
            });
        }

        // 4) Baseline rect re-seat — `SetWindowPos` on a child
        //    window takes parent-client coordinates, so convert the
        //    baseline screen rect's origin through `ScreenToClient`
        //    on the baseline parent. A failed conversion marks the
        //    op unapplied (the unconverted screen origin would land
        //    the window at the wrong offset).
        if let (Some(parent), Some(rect)) = (baseline.parent, baseline.rect) {
            let (applied, error) = unsafe {
                let mut point = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                SetLastError(0);
                let converted = ScreenToClient(parent as HWND, &raw mut point) != 0;
                let moved = SetWindowPos(
                    container as HWND,
                    HWND_TOP,
                    point.x,
                    point.y,
                    i32::try_from(rect.width).unwrap_or(i32::MAX),
                    i32::try_from(rect.height).unwrap_or(i32::MAX),
                    SWP_NOACTIVATE,
                ) != 0;
                let ok = converted && moved;
                let error = if ok { None } else { Some(GetLastError()) };
                (ok, error)
            };
            ops.push(ContainerMutationOutcome {
                api: "SetWindowPos(HWND_TOP,<baseline-rect-parent-client>,SWP_NOACTIVATE)",
                applied,
                win32_error: error,
                after: observe_container(container),
            });
        }

        // 5) Approximate sibling z-order restore — insert directly
        //    below the baseline above-neighbor when that neighbor
        //    is still alive.
        if let Some(above) = baseline.above_hwnd {
            // SAFETY: `IsWindow` is a read-only liveness query on an
            // observed handle before it is reused as insert-after.
            let alive = unsafe { IsWindow(above as HWND) } != 0;
            if alive {
                ops.push(zorder_seat(
                    container,
                    above as HWND,
                    "SetWindowPos(<baseline-above-neighbor>,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)",
                ));
            }
        }

        let final_state = observe_container(container);
        ContainerRestoreOutcome {
            parent_restored: baseline.parent.is_some() && final_state.parent == baseline.parent,
            exstyle_restored: match (&baseline.exstyle, &final_state.exstyle) {
                (Some(baseline), Some(after)) => after.value == baseline.value,
                _ => false,
            },
            rect_restored: match (baseline.rect, final_state.rect) {
                (Some(baseline), Some(after)) => after == baseline,
                _ => false,
            },
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (final_state.above_count, final_state.above_hwnd),
            ops,
            final_state,
        }
    }

    /// The baseline z-band op for [`restore_container`] — `None`
    /// when the baseline exstyle is unknown or the window already
    /// sits in the baseline band. `WS_EX_TOPMOST` is band state —
    /// a `GWL_EXSTYLE` write of the baseline value cannot clear it
    /// (observed: the `0x8 → 0` write returns without error yet the
    /// bit stays set), only `SetWindowPos(HWND_NOTOPMOST)` moves
    /// the window out of the topmost band. Issued before re-docking
    /// because band state is meaningless — and the write
    /// unreliable — under a foreign-thread parent (see the exstyle
    /// note inside `restore_container`).
    fn band_restore_op(
        container: u64,
        baseline: &ContainerSnapshot,
    ) -> Option<ContainerMutationOutcome> {
        let facts = baseline.exstyle.as_ref()?;
        let current = observe_container(container);
        let current_topmost = current.exstyle.as_ref().is_some_and(ExstyleFacts::topmost);
        if current_topmost == facts.topmost() {
            return None;
        }
        let (insert_after, api) = if facts.topmost() {
            (HWND_TOPMOST, "SetWindowPos(HWND_TOPMOST,<baseline-band>)")
        } else {
            (
                HWND_NOTOPMOST,
                "SetWindowPos(HWND_NOTOPMOST,<baseline-band>)",
            )
        };
        Some(zorder_seat(container, insert_after, api))
    }

    /// `SetWindowLongPtrW(GWL_EXSTYLE, value)` on the container —
    /// `SetLastError(0)` first so a legitimate `0` previous style
    /// value does not masquerade as a failure.
    fn exstyle_write(container: u64, value: u64, api: &'static str) -> ContainerMutationOutcome {
        // `GWL_EXSTYLE` values are u32-range by ABI — a value that
        // does not fit `isize` records an honest unapplied op
        // rather than a truncated write.
        let Ok(new) = isize::try_from(value) else {
            return ContainerMutationOutcome {
                api,
                applied: false,
                win32_error: None,
                after: observe_container(container),
            };
        };
        // SAFETY: `SetWindowLongPtrW(GWL_EXSTYLE)` is the
        // intentional, bounded style mutation this module exists
        // for; `GetLastError` disambiguates a failed call.
        let error = unsafe {
            SetLastError(0);
            SetWindowLongPtrW(container as HWND, GWL_EXSTYLE, new);
            let err = GetLastError();
            (err != 0).then_some(err)
        };
        let after = observe_container(container);
        // `applied` claims the requested style state holds, not the
        // API's return convention — a write on a window owned by
        // another thread or whose class forbids the bit can be
        // dropped silently, and a refused call whose target state
        // already holds (baseline exstyle written over itself — the
        // `SetParent`-adjacent windows in the rf532 probe refuse the
        // write with error 6 yet change nothing) is still a state
        // that satisfies the mutation's contract. The refusal stays
        // recorded in `win32_error` either way; only the observed
        // read-back decides `applied`.
        let applied = after.exstyle.as_ref().is_some_and(|e| e.value == value);
        ContainerMutationOutcome {
            api,
            applied,
            win32_error: error,
            after,
        }
    }

    /// `SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|
    /// SWP_NOZORDER|SWP_NOACTIVATE)` — commits a cached exstyle
    /// change without moving, sizing, re-ordering, or activating.
    fn frame_apply(container: u64) -> ContainerMutationOutcome {
        // SAFETY: bounded `SetWindowPos` style-commit mutation.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetWindowPos(
                container as HWND,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ContainerMutationOutcome {
            api: "SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)",
            applied,
            win32_error: error,
            after: observe_container(container),
        }
    }

    /// `SetWindowPos` onto a rect at `insert_after` with `flags` —
    /// the shared seat used by the reparent stage's screen-rect
    /// application.
    fn rect_seat(
        container: u64,
        insert_after: HWND,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        api: &'static str,
    ) -> ContainerMutationOutcome {
        // SAFETY: bounded `SetWindowPos` move/size mutation.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetWindowPos(
                container as HWND,
                insert_after,
                x,
                y,
                i32::try_from(width).unwrap_or(i32::MAX),
                i32::try_from(height).unwrap_or(i32::MAX),
                SWP_NOACTIVATE,
            ) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ContainerMutationOutcome {
            api,
            applied,
            win32_error: error,
            after: observe_container(container),
        }
    }

    /// `SetWindowPos(insert_after, ZORDER_FLAGS)` on the container —
    /// the sibling z-order reposition (container counterpart of the
    /// root's `mutation` helper).
    fn zorder_seat(
        container: u64,
        insert_after: HWND,
        api: &'static str,
    ) -> ContainerMutationOutcome {
        // SAFETY: bounded `SetWindowPos` z-order mutation —
        // `ZORDER_FLAGS` suppresses move/size/activation.
        let (applied, error) = unsafe {
            SetLastError(0);
            let ok = SetWindowPos(container as HWND, insert_after, 0, 0, 0, 0, ZORDER_FLAGS) != 0;
            let error = if ok { None } else { Some(GetLastError()) };
            (ok, error)
        };
        ContainerMutationOutcome {
            api,
            applied,
            win32_error: error,
            after: observe_container(container),
        }
    }

    /// Restore outcome for a baseline with no container handle —
    /// nothing was mutated, so there is nothing to restore; the
    /// fields still report the baseline honestly.
    fn empty_container_restore(baseline: &ContainerSnapshot) -> ContainerRestoreOutcome {
        ContainerRestoreOutcome {
            ops: Vec::new(),
            final_state: baseline.clone(),
            parent_restored: baseline.parent.is_none(),
            exstyle_restored: baseline.exstyle.is_none(),
            rect_restored: baseline.rect.is_none(),
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (baseline.above_count, baseline.above_hwnd),
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
    use super::{
        ContainerMutationOutcome, ContainerRestoreOutcome, ContainerSnapshot, MutationOutcome,
        RestoreOutcome, RootSnapshot, ScreenRect,
    };

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

    fn unsupported_container_snapshot(container: u64) -> ContainerSnapshot {
        ContainerSnapshot {
            supported: false,
            container: Some(container),
            alive: false,
            parent: None,
            exstyle: None,
            class_style: None,
            rect: None,
            above_count: None,
            above_hwnd: None,
            below_hwnd: None,
            truncated: false,
        }
    }

    fn unsupported_container_outcome(
        api: &'static str,
        container: u64,
    ) -> ContainerMutationOutcome {
        ContainerMutationOutcome {
            api,
            applied: false,
            win32_error: None,
            after: unsupported_container_snapshot(container),
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

    pub(super) fn observe_container(container: u64) -> ContainerSnapshot {
        unsupported_container_snapshot(container)
    }

    pub(super) fn repaint_container(container: u64) -> ContainerMutationOutcome {
        unsupported_container_outcome(
            "RedrawWindow(RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW)",
            container,
        )
    }

    /// No HWND/message queue exists off-Windows — report the drain
    /// unsupported with no ops rather than fabricating paint facts.
    pub(super) fn drain_paint(hwnd: u64) -> PaintDrainOutcome {
        PaintDrainOutcome {
            supported: false,
            hwnd,
            update_before: None,
            ops: Vec::new(),
            dispatched: 0,
            update_after: None,
            cleared: false,
        }
    }

    pub(super) fn apply_layered(container: u64) -> Vec<ContainerMutationOutcome> {
        vec![
            unsupported_container_outcome(
                "SetWindowLongPtrW(GWL_EXSTYLE,<current>|WS_EX_LAYERED)",
                container,
            ),
            unsupported_container_outcome(
                "SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)",
                container,
            ),
            unsupported_container_outcome("SetLayeredWindowAttributes(0,255,LWA_ALPHA)", container),
        ]
    }

    pub(super) fn reparent_to_desktop(
        container: u64,
        _baseline_exstyle: u64,
        _rect: ScreenRect,
    ) -> Vec<ContainerMutationOutcome> {
        vec![
            unsupported_container_outcome("SetWindowLongPtrW(GWL_EXSTYLE,<baseline>)", container),
            unsupported_container_outcome(
                "SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)",
                container,
            ),
            unsupported_container_outcome("SetParent(GetDesktopWindow())", container),
            unsupported_container_outcome(
                "SetWindowPos(HWND_TOP,<baseline-screen-rect>,SWP_NOACTIVATE)",
                container,
            ),
        ]
    }

    pub(super) fn restore_container(baseline: &ContainerSnapshot) -> ContainerRestoreOutcome {
        ContainerRestoreOutcome {
            ops: Vec::new(),
            final_state: baseline.clone(),
            parent_restored: baseline.parent.is_none(),
            exstyle_restored: baseline.exstyle.is_none(),
            rect_restored: baseline.rect.is_none(),
            zorder_before: (baseline.above_count, baseline.above_hwnd),
            zorder_after: (baseline.above_count, baseline.above_hwnd),
        }
    }
}
