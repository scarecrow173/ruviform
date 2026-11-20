//! OS presentation observation for the real-window dock lane
//! (Issue #532 presentation-probe slice — feature `real-dock`).
//!
//! Query-only Win32 reads used by the `real-dcomp-present` case to
//! separate *which presentation layer* hides the docked editor
//! child's pixels — the same boundary convention as the sibling
//! `osoccl`/`ospix`/`osmut` modules: this module never creates,
//! destroys, moves, re-styles, re-orders, or messages a window; it
//! only enumerates top-level windows and reads presentation
//! attributes.
//!
//! Two observation families:
//!
//! - [`inventory`] — `EnumWindows` over every top-level window in
//!   z-order. `WindowFromPoint` skips hidden, disabled,
//!   `WS_EX_TRANSPARENT`/layered, and DWM-cloaked windows, so a
//!   hit-test answer of "the child" cannot rule out a visual cover
//!   the hit-test never sees. The inventory records every
//!   top-level window's class/rect/visibility/liveness/disabled/
//!   exstyle/owner/pid so the caller can isolate child-rect
//!   intersections and own-process vs foreign covers.
//! - [`attributes`] — per-HWND presentation attributes:
//!   `DWMWA_CLOAKED` (DWM composition state), `GetWindowRgn` +
//!   `GetRgnBox` (window clip region), `GetUpdateRect` (pending
//!   invalid region), and `GetLayeredWindowAttributes` (layered
//!   alpha/color-key). Each query degrades independently — a
//!   failure is `None` + error detail, never a fabricated value.
//!
//! All values crossing the module boundary are plain integers,
//! booleans, strings, and [`ScreenRect`]s — no Win32 type reaches
//! the caller (the same `u64`-narrowed-handle convention as
//! `osoccl`/`ospix`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

use crate::osoccl::ExstyleFacts;
use crate::ospix::ScreenRect;

/// Safety cap on the top-level window inventory — a real desktop
/// has far fewer top-level windows; hitting it records
/// [`Inventory::truncated`] rather than looping on a corrupted
/// enumeration.
const INVENTORY_CAP: usize = 256;

/// One top-level window's identifying facts — plain values only.
#[derive(Debug, Clone)]
pub(crate) struct TopLevelWindow {
    /// The HWND value as `u64`.
    pub hwnd: u64,
    /// Z-order position in `EnumWindows` enumeration order — 0 is
    /// the topmost top-level window.
    pub z_index: u32,
    /// `GetClassNameW` result — `None` when the query fails.
    pub class_name: Option<String>,
    /// `GetWindowRect` in virtual-screen coordinates — `None` on
    /// query failure or an empty rectangle.
    pub rect: Option<ScreenRect>,
    /// `IsWindowVisible(hwnd) != 0`.
    pub visible: bool,
    /// `IsWindow(hwnd) != 0`.
    pub alive: bool,
    /// `WS_DISABLED` bit of `GWL_STYLE` — `WindowFromPoint`
    /// skips disabled windows (and their subtrees), so this is a
    /// hit-test-relevant state. `None` when the style query fails.
    pub disabled: Option<bool>,
    /// `GWL_EXSTYLE` facts — the compositor/hit-test-relevant bits
    /// (`WS_EX_TRANSPARENT`/`LAYERED`/`NOREDIRECTIONBITMAP`/...).
    /// `None` when the style query fails.
    pub exstyle: Option<ExstyleFacts>,
    /// `GetWindow(GW_OWNER)` — the owner top-level window, `None`
    /// for unowned windows or when the query fails/returns null.
    pub owner: Option<u64>,
    /// `GetWindowThreadProcessId(hwnd)` process id — `None` when
    /// the query fails.
    pub pid: Option<u32>,
    /// The owner window's process id — `None` when there is no
    /// owner or the query fails.
    pub owner_pid: Option<u32>,
}

/// The top-level window inventory — every `EnumWindows` entry in
/// z-order, capped at [`INVENTORY_CAP`].
#[derive(Debug)]
pub(crate) struct Inventory {
    /// `false` only on the non-Windows stub — callers gate on the
    /// attach observation's `supported` before probing, so this is
    /// a defensive flag, not a second oracle.
    pub supported: bool,
    /// `GetCurrentProcessId` — the lane process (the GPUI process
    /// on the real lane). Callers compare each window's `pid`
    /// against this to split own-process vs foreign covers.
    pub own_pid: u32,
    /// The enumerated top-level windows, z-order (topmost first).
    pub windows: Vec<TopLevelWindow>,
    /// The lane process's own top-level windows — every
    /// `EnumWindows` entry whose `pid == own_pid`, regardless of the
    /// [`INVENTORY_CAP`]. `z_index` is the position in the full
    /// enumeration, so the own windows' true z-order is observable
    /// even when the capped `windows` prefix truncated them away
    /// (Issue #532 uncovered-baseline bisect — presentation-probe
    /// 残課題 c).
    pub own_windows: Vec<TopLevelWindow>,
    /// Total top-level windows `EnumWindows` returned before the
    /// cap — always `>= windows.len()`.
    pub total: usize,
    /// `true` when the enumeration exceeded [`INVENTORY_CAP`] —
    /// `windows` is then a prefix, not the whole set.
    pub truncated: bool,
}

/// One window's clip region facts — `GetWindowRgn` + `GetRgnBox`.
#[derive(Debug, Clone)]
pub(crate) struct RegionFacts {
    /// `GetWindowRgn` return value — the raw region type
    /// (`NULLREGION`/`SIMPLEREGION`/`COMPLEXREGION`/`ERROR`),
    /// recorded verbatim so the reader interprets the ABI value.
    pub region_type: i32,
    /// `GetRgnBox` bounding box in *window* coordinates — `None`
    /// when the box query fails or the region is empty.
    pub bounds: Option<ScreenRect>,
    /// `GetWindowRgn` failure detail when `region_type` is the
    /// error sentinel — `None` on success.
    pub error: Option<String>,
}

/// One window's pending invalid region — `GetUpdateRect`.
#[derive(Debug, Clone)]
pub(crate) struct UpdateFacts {
    /// `GetUpdateRect` nonzero result — `true` when an update
    /// region exists (a paint is pending).
    pub has_update: bool,
    /// The update region's bounding box in *client* coordinates —
    /// `None` when there is no update region or the query fails.
    pub rect: Option<ScreenRect>,
    /// `GetUpdateRect` failure detail when the call itself failed —
    /// distinguishes a dead handle from a genuinely empty update
    /// region. `None` on success.
    pub error: Option<String>,
}

/// One window's layered attributes — `GetLayeredWindowAttributes`.
#[derive(Debug, Clone)]
pub(crate) struct LayeredFacts {
    /// `bAlpha` — the layered window's alpha value.
    pub alpha: Option<u8>,
    /// `crKey` — the layered window's color-key.
    pub color_key: Option<u32>,
    /// `dwFlags` — `LWA_*` bits (`LWA_COLORKEY`/`LWA_ALPHA`).
    pub flags: Option<u32>,
}

/// One HWND's presentation attributes — each query family degrades
/// independently so one failed call never hides the others.
#[derive(Debug)]
pub(crate) struct PresentationAttributes {
    /// `false` only on the non-Windows stub.
    pub supported: bool,
    /// `DWMWA_CLOAKED` raw value — DWM cloak bits (shell/app/
    /// inherited). `None` when `DwmGetWindowAttribute` fails or the
    /// attribute is unavailable.
    pub cloaked: Option<u32>,
    /// Window clip region facts — `None` when the query fails.
    pub region: Option<RegionFacts>,
    /// Pending invalid region — `None` when the query fails.
    pub update: Option<UpdateFacts>,
    /// Layered attributes — `None` when the window is not layered
    /// or the query fails (the exstyle record already carries
    /// `WS_EX_LAYERED`).
    pub layered: Option<LayeredFacts>,
}

/// Every top-level window on the desktop in z-order — see the
/// module docs for why `WindowFromPoint` alone cannot rule out a
/// visual cover. Never mutates any window.
pub(crate) fn inventory() -> Inventory {
    platform::inventory()
}

/// Presentation attributes for one HWND — see the module docs.
/// Never mutates any window.
pub(crate) fn attributes(hwnd: u64) -> PresentationAttributes {
    platform::attributes(hwnd)
}

#[cfg(windows)]
#[expect(unsafe_code)]
mod platform {
    use std::ffi::c_void;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, RECT, SetLastError};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateRectRgn, DeleteObject, GetRgnBox, GetUpdateRect, GetWindowRgn,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetLayeredWindowAttributes,
        GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
        WS_DISABLED,
    };

    use super::{
        Inventory, LayeredFacts, PresentationAttributes, RegionFacts, ScreenRect, TopLevelWindow,
        UpdateFacts,
    };
    use crate::osoccl::ExstyleFacts;

    /// `DWMWA_CLOAKED` — the `DwmGetWindowAttribute` attribute id
    /// for the DWM cloak state (stable Win32 ABI value; windows-sys
    /// does not export the DWM API, so the extern is declared
    /// locally below).
    const DWMWA_CLOAKED: u32 = 14;
    /// `HRESULT` success — the only `DwmGetWindowAttribute` result
    /// whose output value is adopted.
    const S_OK: i32 = 0;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        /// `DwmGetWindowAttribute` — local declaration (same
        /// precedent as `PrintWindow` in `ospix`): windows-sys does
        /// not export the DWM API surface.
        fn DwmGetWindowAttribute(
            hwnd: HWND,
            dwattribute: u32,
            pvattribute: *mut c_void,
            cbattribute: u32,
        ) -> i32;
    }

    /// `EnumWindows` collects every top-level HWND in z-order into
    /// a `Vec<u64>` — the callback only appends the handle; all
    /// per-window queries run after enumeration so a slow query
    /// cannot stall the desktop walk.
    pub(super) fn inventory() -> Inventory {
        unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> i32 {
            // SAFETY: `lparam` is the `*mut Vec<u64>` the caller
            // passed; it outlives this synchronous enumeration.
            unsafe { (*(lparam as *mut Vec<u64>)).push(hwnd as usize as u64) };
            1
        }
        let mut found: Vec<u64> = Vec::new();
        // SAFETY: read-only enumeration; `found` outlives the call.
        let enumerated = unsafe { EnumWindows(Some(collect), &raw mut found as LPARAM) != 0 };
        let total = found.len();
        // SAFETY: query-only process-id read.
        let own_pid = unsafe { GetCurrentProcessId() };
        let mut windows: Vec<TopLevelWindow> = Vec::new();
        let mut own_windows: Vec<TopLevelWindow> = Vec::new();
        for (index, hwnd) in found.into_iter().enumerate() {
            let z_index = u32::try_from(index).expect("index fits u32");
            // A single pid query decides whether the full row is
            // needed at all — own-process windows bypass the cap so
            // their true z_index stays observable.
            let mut pid: u32 = 0;
            // SAFETY: read-only pid query; `pid` outlives the call.
            let is_own = unsafe {
                GetWindowThreadProcessId(hwnd as HWND, &raw mut pid) != 0 && pid == own_pid
            };
            if !is_own && windows.len() >= super::INVENTORY_CAP {
                continue;
            }
            let row = top_level_window(hwnd, z_index);
            if is_own {
                own_windows.push(row.clone());
            }
            if windows.len() < super::INVENTORY_CAP {
                windows.push(row);
            }
        }
        let truncated = total > windows.len();
        Inventory {
            supported: enumerated,
            own_pid,
            windows,
            own_windows,
            total,
            truncated,
        }
    }

    /// Presentation attributes for one HWND — every query family
    /// is independent; failures degrade to `None` + error detail.
    /// Each helper carries its own `unsafe` scope.
    pub(super) fn attributes(hwnd: u64) -> PresentationAttributes {
        PresentationAttributes {
            supported: true,
            cloaked: dwm_cloaked(hwnd as HWND),
            region: Some(region_facts(hwnd as HWND)),
            update: Some(update_facts(hwnd as HWND)),
            layered: layered_facts(hwnd as HWND),
        }
    }

    /// One top-level window's facts — class/rect/visibility/
    /// liveness/disabled/exstyle/owner/pid, all query-only.
    fn top_level_window(hwnd: u64, z_index: u32) -> TopLevelWindow {
        let hwnd_t = hwnd as HWND;
        // SAFETY: read-only queries on an HWND `EnumWindows` just
        // returned; a window destroyed in between makes each call
        // fail gracefully and the fact degrades to `None`/`false`.
        unsafe {
            let owner = GetWindow(hwnd_t, GW_OWNER);
            let owner_alive = !owner.is_null();
            let (pid, owner_pid) = {
                let mut pid = 0u32;
                let mut owner_pid = 0u32;
                // A zero thread-id return means the query failed
                // (e.g. the HWND died between enumeration and this
                // read) — recorded as `None`, never `Some(0)`.
                let tid = GetWindowThreadProcessId(hwnd_t, &raw mut pid);
                let owner_thread = if owner_alive {
                    GetWindowThreadProcessId(owner, &raw mut owner_pid)
                } else {
                    0
                };
                (
                    (tid != 0).then_some(pid),
                    (owner_thread != 0).then_some(owner_pid),
                )
            };
            TopLevelWindow {
                hwnd,
                z_index,
                class_name: class_name(hwnd_t),
                rect: crate::ospix::window_screen_rect(hwnd).ok(),
                visible: IsWindowVisible(hwnd_t) != 0,
                alive: IsWindow(hwnd_t) != 0,
                disabled: disabled(hwnd_t),
                exstyle: exstyle(hwnd_t),
                owner: owner_alive.then_some(owner as usize as u64),
                pid,
                owner_pid,
            }
        }
    }

    /// `GWL_EXSTYLE` of `hwnd` — `None` on query failure (a dead
    /// handle also reports `alive:false`, which disambiguates).
    fn exstyle(hwnd: HWND) -> Option<ExstyleFacts> {
        // SAFETY: read-only style query on an observed handle.
        unsafe {
            SetLastError(0);
            let raw = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if GetLastError() != 0 {
                return None;
            }
            u32::try_from(raw).ok().map(|style| ExstyleFacts {
                value: u64::from(style),
            })
        }
    }

    /// `WS_DISABLED` bit of `GWL_STYLE` — `WindowFromPoint` skips
    /// disabled windows, so this is hit-test-relevant state.
    fn disabled(hwnd: HWND) -> Option<bool> {
        // SAFETY: read-only style query on an observed handle.
        unsafe {
            SetLastError(0);
            let raw = GetWindowLongPtrW(hwnd, GWL_STYLE);
            if GetLastError() != 0 {
                return None;
            }
            u32::try_from(raw)
                .ok()
                .map(|style| style & WS_DISABLED != 0)
        }
    }

    /// `GetClassNameW` into a fixed buffer — internal class atoms
    /// read directly, no window message involved.
    fn class_name(hwnd: HWND) -> Option<String> {
        let mut buffer = [0u16; 256];
        // SAFETY: `buffer` is a valid writable buffer; the return
        // is the number of characters copied (0 on failure).
        let len = unsafe {
            GetClassNameW(
                hwnd,
                buffer.as_mut_ptr(),
                i32::try_from(buffer.len()).expect("class-name buffer fits i32"),
            )
        };
        if len <= 0 {
            return None;
        }
        let len = usize::try_from(len).unwrap_or(0).min(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]))
    }

    /// `DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED)` — the DWM
    /// cloak state. `None` on any non-`S_OK` HRESULT or a value
    /// that does not fit `u32`.
    fn dwm_cloaked(hwnd: HWND) -> Option<u32> {
        let mut value: u32 = 0;
        // SAFETY: `value` is a valid `u32` out-param of the size
        // `cbAttribute` declares; `hwnd` is an observed handle.
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                (&raw mut value).cast::<c_void>(),
                u32::try_from(size_of::<u32>()).expect("u32 size fits u32"),
            )
        };
        (hr == S_OK).then_some(value)
    }

    /// `GetWindowRgn` + `GetRgnBox` — the window clip region's
    /// type and bounding box. The temporary region object is
    /// created empty, filled by `GetWindowRgn`, read, and deleted
    /// on every path.
    fn region_facts(hwnd: HWND) -> RegionFacts {
        // SAFETY: `CreateRectRgn` returns a new region object or
        // null; every GDI object below is deleted before return.
        unsafe {
            let region = CreateRectRgn(0, 0, 0, 0);
            if region.is_null() {
                return RegionFacts {
                    region_type: -1,
                    bounds: None,
                    error: Some(last_error("CreateRectRgn")),
                };
            }
            SetLastError(0);
            let region_type = GetWindowRgn(hwnd, region);
            let error = (region_type == 0).then(|| last_error("GetWindowRgn"));
            let mut rect = RECT::default();
            let bounds = if GetRgnBox(region, &raw mut rect) != 0 {
                rect_to_screen(&rect)
            } else {
                None
            };
            // SAFETY: `region` is our live HRGN, deleted exactly
            // once.
            DeleteObject(region.cast());
            RegionFacts {
                region_type,
                bounds,
                error,
            }
        }
    }

    /// `GetUpdateRect(erase = FALSE)` — the pending invalid
    /// region's bounding box in *client* coordinates. A zero return
    /// with a live last-error is a query failure (dead handle),
    /// recorded as `error` — never confused with "no update".
    fn update_facts(hwnd: HWND) -> UpdateFacts {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a valid out-param; `erase=FALSE` keeps
        // the query read-only (no region mutation).
        unsafe {
            SetLastError(0);
            let has_update = GetUpdateRect(hwnd, &raw mut rect, 0) != 0;
            let error = (!has_update && GetLastError() != 0).then(|| last_error("GetUpdateRect"));
            UpdateFacts {
                has_update,
                rect: has_update.then(|| rect_to_screen(&rect)).flatten(),
                error,
            }
        }
    }

    /// `GetLayeredWindowAttributes` — alpha/color-key/flags.
    /// `None` when the call fails (non-layered windows report
    /// failure; `WS_EX_LAYERED` is already in the exstyle record).
    fn layered_facts(hwnd: HWND) -> Option<LayeredFacts> {
        let mut color_key: u32 = 0;
        let mut alpha: u8 = 0;
        let mut flags: u32 = 0;
        // SAFETY: all three are valid out-params of the sizes the
        // API declares; `hwnd` is an observed handle.
        let ok = unsafe {
            GetLayeredWindowAttributes(hwnd, &raw mut color_key, &raw mut alpha, &raw mut flags)
        };
        (ok != 0).then_some(LayeredFacts {
            alpha: Some(alpha),
            color_key: Some(color_key),
            flags: Some(flags),
        })
    }

    /// `RECT` → [`ScreenRect`] — `None` on empty/overflowing rects.
    /// The coordinate space is whatever the caller's query
    /// produced (window rect, region bounds, or client update
    /// rect); the field names carry no coordinate claim.
    fn rect_to_screen(rect: &RECT) -> Option<ScreenRect> {
        let width = rect.right.checked_sub(rect.left)?;
        let height = rect.bottom.checked_sub(rect.top)?;
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(ScreenRect {
            left: rect.left,
            top: rect.top,
            width: u32::try_from(width).ok()?,
            height: u32::try_from(height).ok()?,
        })
    }

    /// `GetLastError` as a message string naming the failing call.
    fn last_error(call: &str) -> String {
        // SAFETY: `GetLastError` reads the calling thread's error
        // value; no preconditions.
        let code = unsafe { GetLastError() };
        format!("{call} failed (win32 error {code})")
    }
}

/// Non-Windows stub — the `NativePluginDock` capability (and thus
/// the case that calls this) is Windows-only, so this path is
/// unreachable in practice; it exists so the crate compiles on
/// every target and records `supported: false` with empty fields,
/// never fabricated observations.
#[cfg(not(windows))]
mod platform {
    use super::{Inventory, PresentationAttributes};

    pub(super) fn inventory() -> Inventory {
        Inventory {
            supported: false,
            own_pid: 0,
            windows: Vec::new(),
            own_windows: Vec::new(),
            total: 0,
            truncated: false,
        }
    }

    pub(super) fn attributes(_hwnd: u64) -> PresentationAttributes {
        PresentationAttributes {
            supported: false,
            cloaked: None,
            region: None,
            update: None,
            layered: None,
        }
    }
}
