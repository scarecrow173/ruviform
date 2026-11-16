//! OS occlusion diagnostics for the real-window dock lane (Issue
//! #532 dcomp-occlusion slice — feature `real-dock`).
//!
//! Query-only Win32 window-tree reads used by the
//! `real-dcomp-pixel` case to separate *which layer* hides the
//! docked editor child's pixels — the same boundary convention as
//! the sibling `oschild`/`ospix` modules: this module never
//! creates, destroys, moves, re-styles, re-orders, or messages a
//! window; it only reads ancestry, styles, z-order, rectangles,
//! and hit-test results.
//!
//! Three observation families per docked child HWND:
//!
//! - Root extended style — `GetAncestor(GA_ROOT)` →
//!   `GetWindowLongPtrW(GWL_EXSTYLE)`. `WS_EX_NOREDIRECTIONBITMAP`
//!   is GPUI's `DComp`-path marker (present only when
//!   `GPUI_DISABLE_DIRECT_COMPOSITION` is unset), so its presence
//!   per run proves whether the env switch reached GPUI's window
//!   creation.
//! - Sibling z-order — `GetWindow(GW_CHILD)`/`GW_HWNDNEXT` walks
//!   the direct children of the child's parent (the container)
//!   and of the root (the GPUI top-level), topmost first. Any
//!   HWND-level cover shows up here as a sibling above the child
//!   or container.
//! - `WindowFromPoint` — five representative points inside the
//!   child's screen rectangle resolve to the HWND the window
//!   manager considers topmost there. A non-child answer is
//!   HWND-level occlusion evidence; a child answer with zero
//!   screen green points at compositor-level occlusion (`DComp`
//!   visual or flip-model presentation plane) — though the
//!   hit-test's determinism under GPU composition is itself an
//!   observed limitation, recorded as such.
//!
//! All values crossing the module boundary are plain integers,
//! booleans, strings, and [`ScreenRect`]s — no Win32 type reaches
//! the caller (the same `u64`-narrowed-handle convention as
//! `oschild`/`ospix`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

use crate::ospix::ScreenRect;

/// `WS_EX_*` extended-window-style bits — stable Win32 ABI
/// values, declared platform-neutrally (as `u64`) so
/// [`ExstyleFacts`] and its accessors compile on every target;
/// they are the same bits `windows-sys` exposes as `u32`
/// `WINDOW_EX_STYLE` constants.
const WS_EX_TOPMOST: u64 = 0x0000_0008;
const WS_EX_TRANSPARENT: u64 = 0x0000_0020;
const WS_EX_TOOLWINDOW: u64 = 0x0000_0080;
const WS_EX_APPWINDOW: u64 = 0x0004_0000;
const WS_EX_LAYERED: u64 = 0x0008_0000;
const WS_EX_NOREDIRECTIONBITMAP: u64 = 0x0020_0000;
const WS_EX_NOACTIVATE: u64 = 0x0800_0000;

/// One window's identifying facts — plain values only.
#[derive(Debug, Clone)]
pub(crate) struct WindowRow {
    /// The HWND value as `u64`.
    pub hwnd: u64,
    /// Sibling z-order position (0 = topmost) when the row came
    /// from a [`ZOrder`] walk; `None` for hit-test/standalone rows.
    pub z_index: Option<u32>,
    /// `GetClassNameW` result — `None` when the query fails.
    pub class_name: Option<String>,
    /// `GetWindowRect` in virtual-screen coordinates — `None` on
    /// query failure or an empty rectangle.
    pub rect: Option<ScreenRect>,
    /// `IsWindowVisible(hwnd) != 0`.
    pub visible: bool,
    /// `IsWindow(hwnd) != 0`.
    pub alive: bool,
}

/// One window's raw `GWL_EXSTYLE` value with named accessors for
/// the compositor-relevant bits — the flags derive from `value`,
/// so the raw DWORD stays the single recorded fact.
#[derive(Debug, Clone)]
pub(crate) struct ExstyleFacts {
    /// The raw extended-style value (`u64`-narrowed).
    pub value: u64,
}

impl ExstyleFacts {
    /// `WS_EX_NOREDIRECTIONBITMAP` — GPUI's `DComp`-path marker;
    /// absent only when `disable_direct_composition` was set at
    /// GPUI initialization.
    pub fn noredirectionbitmap(&self) -> bool {
        self.value & WS_EX_NOREDIRECTIONBITMAP != 0
    }

    /// `WS_EX_TOPMOST`.
    pub fn topmost(&self) -> bool {
        self.value & WS_EX_TOPMOST != 0
    }

    /// `WS_EX_LAYERED`.
    pub fn layered(&self) -> bool {
        self.value & WS_EX_LAYERED != 0
    }

    /// `WS_EX_TRANSPARENT`.
    pub fn transparent(&self) -> bool {
        self.value & WS_EX_TRANSPARENT != 0
    }

    /// `WS_EX_NOACTIVATE`.
    pub fn noactivate(&self) -> bool {
        self.value & WS_EX_NOACTIVATE != 0
    }

    /// `WS_EX_TOOLWINDOW`.
    pub fn toolwindow(&self) -> bool {
        self.value & WS_EX_TOOLWINDOW != 0
    }

    /// `WS_EX_APPWINDOW`.
    pub fn appwindow(&self) -> bool {
        self.value & WS_EX_APPWINDOW != 0
    }
}

/// Direct children of one window in z-order — index 0 is topmost.
#[derive(Debug, Clone)]
pub(crate) struct ZOrder {
    /// The children, topmost first.
    pub children: Vec<WindowRow>,
    /// `true` when the walk hit the safety cap before the sibling
    /// chain ended — the list is then a prefix, not the whole set.
    pub truncated: bool,
}

/// One `WindowFromPoint` probe result at a representative point
/// inside the child rectangle.
#[derive(Debug, Clone)]
pub(crate) struct PointHit {
    /// Probe x, virtual-screen coordinates.
    pub x: i32,
    /// Probe y, virtual-screen coordinates.
    pub y: i32,
    /// The window the hit-test resolved to — `None` when
    /// `WindowFromPoint` returned NULL (no window at the point).
    pub window: Option<WindowRow>,
}

/// Everything [`probe`] observed for one docked child HWND — each
/// part degrades independently so one failed query never hides
/// the others.
#[derive(Debug)]
pub(crate) struct OcclusionFacts {
    /// `false` only on the non-Windows stub — the caller gates on
    /// `ChildWindowObservation::supported` before probing, so this
    /// is a defensive flag, not a second oracle.
    pub supported: bool,
    /// `GetAncestor(child, GA_ROOT)` — the GPUI top-level window.
    pub root: Option<u64>,
    /// The root's `GWL_EXSTYLE` facts.
    pub root_exstyle: Option<ExstyleFacts>,
    /// Why [`Self::root_exstyle`] is `None` despite a live root —
    /// the `GetWindowLongPtrW` failure detail.
    pub root_exstyle_error: Option<String>,
    /// The root's direct children in z-order — the container's
    /// siblings (including any HWND GPUI itself owns). `None`
    /// when there is no root to enumerate.
    pub root_children: Option<ZOrder>,
    /// `GetParent(child)` — normally the container token.
    pub parent: Option<u64>,
    /// The parent's direct children in z-order — the child's
    /// siblings under the container. `None` when there is no
    /// parent to enumerate.
    pub parent_children: Option<ZOrder>,
    /// `WindowFromPoint` results at the representative points
    /// inside the child's screen rectangle.
    pub hits: Vec<PointHit>,
    /// The child's screen rectangle the probe points were derived
    /// from — `None` when the rect query failed (the earlier
    /// `screen_rect` step row carries the error detail).
    pub child_rect: Option<ScreenRect>,
}

/// Query-only occlusion diagnostics for one docked child HWND —
/// see the module docs for the three observation families. Never
/// mutates any window.
pub(crate) fn probe(child: u64) -> OcclusionFacts {
    platform::probe(child)
}

#[cfg(windows)]
#[expect(unsafe_code)]
mod platform {
    use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, SetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GW_CHILD, GW_HWNDNEXT, GWL_EXSTYLE, GetAncestor, GetClassNameW, GetParent,
        GetWindow, GetWindowLongPtrW, IsWindow, IsWindowVisible, WindowFromPoint,
    };

    use super::{ExstyleFacts, OcclusionFacts, PointHit, ScreenRect, WindowRow, ZOrder};

    /// Safety cap on a sibling z-order walk — a real child list
    /// cannot legitimately approach this; hitting it records
    /// `truncated` rather than looping on a corrupted chain.
    const ZORDER_CAP: usize = 1024;

    /// All reads for one child HWND — ancestry, styles, both
    /// z-order walks, and the `WindowFromPoint` probes. Every call
    /// is a read-only query on an observed-live handle.
    pub(super) fn probe(child: u64) -> OcclusionFacts {
        let child_hwnd = child as HWND;
        // SAFETY: `GetAncestor`/`GetParent`/`GetWindowLongPtrW`/
        // `GetWindow`/`IsWindow*`/`WindowFromPoint` are all
        // read-only queries; no window is created, moved, styled,
        // re-ordered, or messaged.
        unsafe {
            let root = GetAncestor(child_hwnd, GA_ROOT);
            let root_alive = !root.is_null();
            let (root_exstyle, root_exstyle_error) = if root_alive {
                match exstyle(root) {
                    Ok(facts) => (Some(facts), None),
                    Err(error) => (None, Some(error)),
                }
            } else {
                (None, None)
            };
            let root_children = root_alive.then(|| z_order(root));
            let parent = GetParent(child_hwnd);
            let parent_alive = !parent.is_null();
            let parent_children = parent_alive.then(|| z_order(parent));
            let child_rect = crate::ospix::window_screen_rect(child).ok();
            let hits = child_rect.map_or_else(Vec::new, hit_tests);
            OcclusionFacts {
                supported: true,
                root: root_alive.then_some(root as usize as u64),
                root_exstyle,
                root_exstyle_error,
                root_children,
                parent: parent_alive.then_some(parent as usize as u64),
                parent_children,
                hits,
                child_rect,
            }
        }
    }

    /// `GWL_EXSTYLE` of `hwnd` as a raw `u64`-narrowed value.
    /// `SetLastError(0)` disambiguates a legitimate `0` style from
    /// a failed call.
    fn exstyle(hwnd: HWND) -> Result<ExstyleFacts, String> {
        // SAFETY: read-only style query on an observed-live handle.
        unsafe {
            SetLastError(0);
            let raw = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let error = GetLastError();
            if error != 0 {
                return Err(format!(
                    "GetWindowLongPtrW(GWL_EXSTYLE) failed (win32 error {error})"
                ));
            }
            // The extended style is a 32-bit value; a result that
            // does not fit is reported, never silently truncated.
            let style =
                u32::try_from(raw).map_err(|_| "GWL_EXSTYLE value does not fit u32".to_string())?;
            Ok(ExstyleFacts {
                value: u64::from(style),
            })
        }
    }

    /// `parent`'s direct children in z-order, topmost first —
    /// `GetWindow(GW_CHILD)` returns the topmost child and
    /// `GW_HWNDNEXT` walks down the sibling chain. The walk cannot
    /// fail: a NULL first child simply yields an empty list.
    fn z_order(parent: HWND) -> ZOrder {
        let mut children = Vec::new();
        // SAFETY: read-only sibling walk — `GetWindow` never
        // mutates window state.
        let truncated = unsafe {
            let mut current = GetWindow(parent, GW_CHILD);
            while !current.is_null() && children.len() < ZORDER_CAP {
                children.push(window_row(
                    current,
                    Some(u32::try_from(children.len()).expect("z-order index fits u32")),
                ));
                current = GetWindow(current, GW_HWNDNEXT);
            }
            !current.is_null()
        };
        ZOrder {
            children,
            truncated,
        }
    }

    /// Identifying facts for one window — class name, screen
    /// rect, visibility, liveness. `z_index` is carried through
    /// from the z-order walk (`None` elsewhere).
    fn window_row(hwnd: HWND, z_index: Option<u32>) -> WindowRow {
        WindowRow {
            hwnd: hwnd as usize as u64,
            z_index,
            class_name: class_name(hwnd),
            rect: crate::ospix::window_screen_rect(hwnd as usize as u64).ok(),
            // SAFETY: read-only visibility/alive queries.
            visible: unsafe { IsWindowVisible(hwnd) } != 0,
            alive: unsafe { IsWindow(hwnd) } != 0,
        }
    }

    /// `GetClassNameW` into a fixed buffer — internal class atoms
    /// read directly, no window message involved (unlike
    /// `GetWindowTextW`, which can send `WM_GETTEXT`).
    fn class_name(hwnd: HWND) -> Option<String> {
        let mut buffer = [0u16; 256];
        // SAFETY: `buffer` is a valid writable buffer of
        // `buffer.len()` u16 code units; the return is the number
        // of characters copied (0 on failure).
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

    /// `WindowFromPoint` at the five representative points inside
    /// `rect` — center plus the four interior quarter points, so a
    /// partially covered child cannot hide behind a lucky center
    /// probe.
    fn hit_tests(rect: ScreenRect) -> Vec<PointHit> {
        let w = i32::try_from(rect.width).expect("rect width fits i32");
        let h = i32::try_from(rect.height).expect("rect height fits i32");
        let (x0, y0) = (rect.left, rect.top);
        [
            (x0 + w / 2, y0 + h / 2),
            (x0 + w / 4, y0 + h / 4),
            (x0 + w - w / 4, y0 + h / 4),
            (x0 + w / 4, y0 + h - h / 4),
            (x0 + w - w / 4, y0 + h - h / 4),
        ]
        .into_iter()
        .map(|(x, y)| {
            // SAFETY: read-only hit-test query on a plain
            // screen-coordinate pair.
            let hit = unsafe { WindowFromPoint(POINT { x, y }) };
            PointHit {
                x,
                y,
                window: if hit.is_null() {
                    None
                } else {
                    Some(window_row(hit, None))
                },
            }
        })
        .collect()
    }
}

/// Non-Windows stub — the `NativePluginDock` capability (and thus
/// the case that calls this) is Windows-only, so this path is
/// unreachable in practice; it exists so the crate compiles on
/// every target and records `supported: false` with empty fields,
/// never fabricated observations.
#[cfg(not(windows))]
mod platform {
    use super::OcclusionFacts;

    pub(super) fn probe(_child: u64) -> OcclusionFacts {
        OcclusionFacts {
            supported: false,
            root: None,
            root_exstyle: None,
            root_exstyle_error: None,
            root_children: None,
            parent: None,
            parent_children: None,
            hits: Vec::new(),
            child_rect: None,
        }
    }
}
