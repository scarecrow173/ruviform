//! OS-level window pixel capture for golden verification
//! (Issue #314 slice 3 — design
//! `docs/designs/issue-314-golden-capture.md`).
//!
//! [`GpuiDriver::capture_window`] reads the client-area pixels of a
//! window the driver opened through [`GpuiDriver::run`] — the real-OS
//! counterpart of the scenario contract's `capture_frame`, which
//! stays `Unsupported` under `gpui-test` because the test platform
//! has no OS window and no pixel readback (see
//! `gpui_driver/scenario.rs`). This module is the only place GPUI
//! window internals and OS handles meet pixel data; nothing here
//! leaks into `ruviform-ui-test`'s driver-neutral contract.
//!
//! Honesty contract:
//!
//! - Pixels come only from the OS compositor path — never from
//!   semantic state, draw plans, or observer output.
//! - Windows that cannot produce pixels (unknown/destroyed id,
//!   not-yet-opened window, test platform without an OS handle,
//!   minimized/hidden window, zero client area, failed GDI call)
//!   return [`UiError`] — the caller decides how to record
//!   `not_run`/`unsupported`; this layer never fabricates a frame.
//! - The Windows backend prefers `PrintWindow` — the window's own
//!   rendered client area, immune to cursor/tooltip/occlusion noise —
//!   and falls back to a screen `BitBlt` of the displayed client rect
//!   when `PrintWindow` fails at the API level. The fallback measures
//!   what the user actually sees; ambient desktop content can appear
//!   in it, which is a real measurement, not a fabrication.
//!
//! **RT contract:** non-RT only — UI thread (the driver state is
//! `Rc`-based and `run` owns the platform loop).

use ruviform_ui::{UiError, WindowId};

use super::GpuiDriver;

/// OS-window geometry for real-OS input targeting (Issue #313):
/// where the window's client area sits on the physical screen and
/// the scale factor that maps window-logical coordinates onto it.
///
/// This is the measurement an OS-level pointer injection must be
/// aimed through — [`crate::gpui_driver::os_input`] coordinates are
/// physical screen pixels, while [`GpuiDriver::component_bounds`]
/// reports window-logical units; `origin + logical * scale_factor`
/// is the honest conversion, never a hard-coded position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsWindowGeometry {
    /// Client-area left edge in physical screen pixels
    /// (virtual-desktop coordinates — may be negative).
    pub origin_x: i32,
    /// Client-area top edge in physical screen pixels.
    pub origin_y: i32,
    /// Client-area width in physical pixels.
    pub width: u32,
    /// Client-area height in physical pixels.
    pub height: u32,
    /// Window scale factor (logical → physical) at query time.
    pub scale_factor: f64,
}

/// `raw_parent` sentinel recorded on Linux (see
/// [`super::surface::raw_parent_of`]): gpui 0.2.2's X11
/// `PlatformWindow` does not implement `HasWindowHandle` — the call
/// panics — so the X11 window id cannot be captured at open. The
/// X11 backend resolves it lazily: the process's own toplevel is
/// the window gpui tagged `_NET_WM_PID` = our pid at open.
/// `usize::MAX` cannot collide with a real handle (XIDs are 29-bit
/// resource ids; HWND/NSView never reach it either).
#[cfg(target_os = "linux")]
pub(super) const X11_RESOLVE_BY_PID: usize = usize::MAX;

/// One captured frame: RGBA8 pixels, row-major, top-down.
///
/// `pixels.len() == width * height * 4`. This is a plain driver-side
/// DTO — converting it into the golden contract's `RgbaImage` is
/// the caller's one-line step (`ruviform-ui-gpui` does not depend on
/// `ruviform-ui-test` outside `gpui-test`).
#[derive(Debug, Clone, PartialEq)]
pub struct OsCapturedFrame {
    /// Frame width in physical pixels.
    pub width: u32,
    /// Frame height in physical pixels.
    pub height: u32,
    /// Window scale factor (DPI) at capture time, as reported by the
    /// platform — the value golden `GoldenKey::scale_factor` /
    /// `EnvironmentMeta::scale_factor` must record for baseline
    /// identity. Physical pixels are *not* normalized by it.
    pub scale_factor: f64,
    /// Row-major RGBA8, top-down rows. Alpha is always `255`: the
    /// captured surface is opaque window content, so the DIB alpha
    /// byte carries no information and is normalized away.
    pub pixels: Vec<u8>,
    /// Which OS path produced these pixels — evidence for golden
    /// `EnvironmentMeta::backend_detail`. `"win32-printwindow"` is
    /// the window's own rendered client area; `"win32-bitblt-screen"`
    /// is the displayed screen rect (fallback, may include ambient
    /// desktop content such as the cursor); `"x11-getimage"` is the
    /// X11 `GetImage` readback of the window drawable (Xvfb virtual
    /// display in the CI offscreen lane).
    pub backend: &'static str,
}

impl GpuiDriver {
    /// Captures `window`'s client-area pixels through the OS.
    ///
    /// The window must have been opened by [`GpuiDriver::run`] on the
    /// real `gpui` backend — under `gpui-test` (or before the deferred
    /// open is applied) there is no OS surface and this returns
    /// [`UiError::Unsupported`]/[`UiError::Host`] rather than pixels.
    ///
    /// # Errors
    /// - [`UiError::Host`] — unknown window id, window not yet open,
    ///   dead OS handle, minimized/hidden window, empty client area,
    ///   or a failed OS capture call.
    /// - [`UiError::Unsupported`] — the window has no OS handle
    ///   (`gpui-test` platform) or the host OS has no capture backend
    ///   in this slice (macOS and other non-Windows/non-Linux
    ///   targets).
    pub fn capture_window(&self, window: WindowId) -> Result<OsCapturedFrame, UiError> {
        let (opened, raw_parent, scale_factor) = {
            let inner = self.inner.borrow();
            let Some(slot) = inner.windows.get(&window) else {
                return Err(UiError::host(format!("no window {window}")));
            };
            (
                slot.handle.is_some(),
                slot.raw_parent,
                f64::from(slot.scale_factor),
            )
        };
        if !opened {
            return Err(UiError::host(format!(
                "window {window} has no OS window yet — capture after Ready"
            )));
        }
        let Some(raw_parent) = raw_parent else {
            return Err(UiError::unsupported(
                "visual_capture",
                "window has no OS handle — real-OS capture requires the `gpui` backend, not `gpui-test`",
            ));
        };
        backend::capture_client_area(raw_parent, scale_factor)
    }

    /// Reports `window`'s client-area geometry on the physical
    /// screen — the target space of [`crate::gpui_driver::os_input`]
    /// absolute pointer injection (Issue #313).
    ///
    /// Same honesty contract as [`Self::capture_window`]: unknown
    /// or not-yet-opened windows are [`UiError::Host`], windows
    /// without an OS handle (`gpui-test`) are
    /// [`UiError::Unsupported`], and OSes without a backend in this
    /// slice (macOS and other non-Windows/non-Linux targets) report
    /// the same.
    ///
    /// # Errors
    /// - [`UiError::Host`] — unknown window id, window not yet open,
    ///   dead OS handle, or a failed OS geometry call.
    /// - [`UiError::Unsupported`] — no OS handle (`gpui-test`) or no
    ///   backend on this OS.
    pub fn window_geometry(&self, window: WindowId) -> Result<OsWindowGeometry, UiError> {
        let (opened, raw_parent, scale_factor) = {
            let inner = self.inner.borrow();
            let Some(slot) = inner.windows.get(&window) else {
                return Err(UiError::host(format!("no window {window}")));
            };
            (
                slot.handle.is_some(),
                slot.raw_parent,
                f64::from(slot.scale_factor),
            )
        };
        if !opened {
            return Err(UiError::host(format!(
                "window {window} has no OS window yet — query after Ready"
            )));
        }
        let Some(raw_parent) = raw_parent else {
            return Err(UiError::unsupported(
                "window_geometry",
                "window has no OS handle — real-OS geometry requires the `gpui` backend, not `gpui-test`",
            ));
        };
        backend::client_geometry(raw_parent, scale_factor)
    }
}

// Per-OS capture backend. Each returns an [`OsCapturedFrame`] or an
// honest [`UiError`]; adding an OS means implementing this one fn.
#[cfg(windows)]
#[expect(unsafe_code)]
mod backend {
    //! Win32 capture of a window's client area.
    //!
    //! Primary path: `PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT)`
    //! asks the window (or DWM, for composed content) to render its
    //! client area into a caller DC — the window's *own* pixels,
    //! immune to the mouse cursor, tooltips, occlusion, and other
    //! desktop noise that a screen read would pick up.
    //!
    //! Fallback path: `BitBlt` of the client rect from the screen DC
    //! — used only when `PrintWindow` fails at the API level (for
    //! example a window class that cannot service `WM_PRINT`). It
    //! measures what is *displayed*, so ambient desktop content can
    //! leak in; that is documented, real-pixel behavior, not a
    //! fabrication.
    //!
    //! `PrintWindow` is declared as a local `extern "system"` fn:
    //! windows-sys exposes it only under `Win32_Storage_Xps`, a
    //! feature change outside this slice's file boundary. user32 is
    //! already linked through the enabled `Win32_UI_*` features.

    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr;

    use ruviform_ui::UiError;
    use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleDC,
        CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HDC, HGDIOBJ, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, IsIconic, IsWindow, IsWindowVisible,
    };

    use super::{OsCapturedFrame, OsWindowGeometry};

    /// `PrintWindow` `nflags`: client area only, full rendered
    /// content (`PW_CLIENTONLY | PW_RENDERFULLCONTENT`). The Win32
    /// ABI values are stable; windows-sys gates `PW_CLIENTONLY`
    /// behind the disabled `Win32_Storage_Xps` feature.
    const PRINT_WINDOW_CLIENT_FULL: u32 = 0x1 | 0x2;

    #[link(name = "user32")]
    unsafe extern "system" {
        /// `PrintWindow` — see the module docs for why this is a
        /// local declaration rather than a windows-sys import.
        fn PrintWindow(hwnd: HWND, hdcblt: HDC, nflags: u32) -> i32;
    }

    /// Screen-DC handle released on drop — keeps the GDI cleanup
    /// paired with `GetDC` on every exit path.
    struct ScreenDc(HDC);

    impl Drop for ScreenDc {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a live screen DC from `GetDC`;
            // `ReleaseDC` with a null window releases it exactly once.
            unsafe { ReleaseDC(ptr::null_mut(), self.0) };
        }
    }

    /// Captures the client area of the OS window `raw_parent`
    /// (Windows: an `HWND` value captured at open).
    ///
    /// # Errors
    /// [`UiError::Host`] on dead handle, minimized/hidden window,
    /// empty client rect, or any failed GDI call.
    pub(super) fn capture_client_area(
        raw_parent: usize,
        scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        let hwnd = raw_parent as HWND;
        // SAFETY: `hwnd` is the OS handle the driver captured from
        // gpui's own window at open; `IsWindow` is the cheap liveness
        // probe before any other call touches it.
        unsafe {
            if IsWindow(hwnd) == 0 {
                return Err(UiError::host("window OS handle is no longer valid"));
            }
            if IsIconic(hwnd) != 0 {
                return Err(UiError::host(
                    "window is minimized — no client pixels on screen",
                ));
            }
            if IsWindowVisible(hwnd) == 0 {
                return Err(UiError::host(
                    "window is not visible — no client pixels on screen",
                ));
            }
        }

        // SAFETY: `hwnd` passed the `IsWindow` probe above; all calls
        // only write caller-owned out-params.
        let (x, y, w, h) = unsafe { client_screen_rect(hwnd)? };
        match print_window_capture(hwnd, w, h, scale_factor) {
            Ok(frame) => Ok(frame),
            // `PrintWindow` is preferred but not guaranteed — a window
            // class that cannot service `WM_PRINT` fails the call.
            // The screen path still returns real displayed pixels;
            // its `backend` tag records which path produced the frame.
            Err(_) => capture_screen_rect(x, y, w, h, scale_factor),
        }
    }

    /// Client-area screen geometry of the OS window `raw_parent`
    /// (Windows: an `HWND` value captured at open) — the input
    /// targeting query, sharing the capture path's handle probes.
    ///
    /// # Errors
    /// [`UiError::Host`] on dead handle or a failed geometry call.
    pub(super) fn client_geometry(
        raw_parent: usize,
        scale_factor: f64,
    ) -> Result<OsWindowGeometry, UiError> {
        let hwnd = raw_parent as HWND;
        // SAFETY: `hwnd` is the OS handle the driver captured from
        // gpui's own window at open; `IsWindow` is the cheap liveness
        // probe before any other call touches it.
        unsafe {
            if IsWindow(hwnd) == 0 {
                return Err(UiError::host("window OS handle is no longer valid"));
            }
            // SAFETY: `hwnd` passed the `IsWindow` probe above;
            // `client_screen_rect` writes only caller-owned out-params.
            let (x, y, w, h) = client_screen_rect(hwnd)?;
            Ok(OsWindowGeometry {
                origin_x: x,
                origin_y: y,
                width: w,
                height: h,
                scale_factor,
            })
        }
    }

    /// Client rect of `hwnd` mapped to virtual-screen coordinates.
    ///
    /// # Errors
    /// [`UiError::Host`] when `GetClientRect`/`ClientToScreen` fail or
    /// the client area is empty.
    ///
    /// # Safety
    /// `hwnd` must be a live window handle.
    unsafe fn client_screen_rect(hwnd: HWND) -> Result<(i32, i32, u32, u32), UiError> {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a valid out-param; `hwnd` is live (caller
        // probed `IsWindow`).
        if unsafe { GetClientRect(hwnd, &raw mut rect) } == 0 {
            return Err(last_error("GetClientRect"));
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return Err(UiError::host("window client area is empty"));
        }
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: `origin` is a valid out-param; `hwnd` is live.
        if unsafe { ClientToScreen(hwnd, &raw mut origin) } == 0 {
            return Err(last_error("ClientToScreen"));
        }
        Ok((
            origin.x,
            origin.y,
            u32::try_from(w).map_err(|_| UiError::host("client width overflows u32"))?,
            u32::try_from(h).map_err(|_| UiError::host("client height overflows u32"))?,
        ))
    }

    /// `PrintWindow`s `hwnd`'s client area into a fresh top-down
    /// 32bpp DIB sized to the client rect.
    ///
    /// # Errors
    /// [`UiError::Host`] when `PrintWindow` reports failure or the
    /// DIB/GDI path fails — the caller falls back to the screen path.
    fn print_window_capture(
        hwnd: HWND,
        w: u32,
        h: u32,
        scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        // SAFETY: null HWND requests the entire screen DC — it only
        // supplies the pixel format for the memory DC/DIB.
        let screen = unsafe { GetDC(ptr::null_mut()) };
        if screen.is_null() {
            return Err(last_error("GetDC"));
        }
        let screen = ScreenDc(screen);

        // SAFETY: `screen.0` is a live screen DC; `hwnd` is live
        // (caller probed `IsWindow`); every GDI object below is
        // created, used, and destroyed within this scope.
        unsafe {
            let mem = CreateCompatibleDC(screen.0);
            if mem.is_null() {
                return Err(last_error("CreateCompatibleDC"));
            }
            let result = dib_capture(
                mem,
                screen.0,
                w,
                h,
                scale_factor,
                "win32-printwindow",
                |dc| {
                    // SAFETY: `dc` is a live memory DC with our DIB
                    // selected; `hwnd` is live; the flags request the
                    // client area rendered with full content.
                    if PrintWindow(hwnd, dc, PRINT_WINDOW_CLIENT_FULL) != 0 {
                        Ok(())
                    } else {
                        Err(last_error("PrintWindow"))
                    }
                },
            );
            // SAFETY: `mem` is our live compatible DC.
            DeleteDC(mem);
            result
        }
    }

    /// `BitBlt`s the screen rect `(x, y, w, h)` into a fresh top-down
    /// 32bpp DIB and normalizes BGRA → RGBA8.
    ///
    /// # Errors
    /// [`UiError::Host`] on any failed GDI call.
    fn capture_screen_rect(
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        // SAFETY: null HWND requests the entire screen DC.
        let screen = unsafe { GetDC(ptr::null_mut()) };
        if screen.is_null() {
            return Err(last_error("GetDC"));
        }
        let screen = ScreenDc(screen);

        // SAFETY: `screen.0` is a live screen DC; every object below
        // is created, used, and destroyed within this scope.
        unsafe {
            let mem = CreateCompatibleDC(screen.0);
            if mem.is_null() {
                return Err(last_error("CreateCompatibleDC"));
            }
            let result = dib_capture(
                mem,
                screen.0,
                w,
                h,
                scale_factor,
                "win32-bitblt-screen",
                |dc| {
                    // SAFETY: `dc` is a live memory DC with our DIB
                    // selected; `screen` is a live screen DC; the src
                    // rect is `(x, y, w, h)` on the virtual screen.
                    if BitBlt(
                        dc,
                        0,
                        0,
                        i32::try_from(w).expect("client width fits i32"),
                        i32::try_from(h).expect("client height fits i32"),
                        screen.0,
                        x,
                        y,
                        SRCCOPY,
                    ) != 0
                    {
                        Ok(())
                    } else {
                        Err(last_error("BitBlt"))
                    }
                },
            );
            // SAFETY: `mem` is our live compatible DC.
            DeleteDC(mem);
            result
        }
    }

    /// Creates the DIB, runs `render` against the selected memory DC,
    /// converts BGRA → RGBA8, and destroys the bitmap — factored out
    /// so both capture paths share the DIB plumbing.
    ///
    /// # Safety
    /// `mem`/`ref_dc` must be live DCs; `render` must only draw into
    /// the memory DC it is given; all GDI objects are local.
    unsafe fn dib_capture(
        mem: HDC,
        ref_dc: HDC,
        w: u32,
        h: u32,
        scale_factor: f64,
        backend: &'static str,
        render: impl FnOnce(HDC) -> Result<(), UiError>,
    ) -> Result<OsCapturedFrame, UiError> {
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize =
            u32::try_from(size_of::<BITMAPINFOHEADER>()).expect("BITMAPINFOHEADER size fits u32");
        bmi.bmiHeader.biWidth = i32::try_from(w).expect("client width fits i32");
        // Negative height requests a top-down DIB — rows arrive in
        // display order without a flip pass.
        bmi.bmiHeader.biHeight = -i32::try_from(h).expect("client height fits i32");
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = ptr::null_mut();
        // SAFETY: `bmi` is a valid BITMAPINFO for a 32bpp BI_RGB DIB;
        // `ref_dc` is a live screen DC supplying the pixel format;
        // `bits` receives the section's pixel pointer.
        let bitmap = unsafe {
            CreateDIBSection(
                ref_dc,
                &raw const bmi,
                DIB_RGB_COLORS,
                &raw mut bits,
                ptr::null_mut(),
                0,
            )
        };
        if bitmap.is_null() {
            return Err(last_error("CreateDIBSection"));
        }

        // SAFETY: `bitmap` is our live DIB section; `mem` is a live
        // compatible DC. The previous stock object is restored before
        // the section is deleted.
        let result = unsafe {
            let stock = SelectObject(mem, bitmap);
            let rendered = render(mem);
            SelectObject(mem, stock);
            match rendered {
                Err(e) => Err(e),
                Ok(()) if bits.is_null() => {
                    Err(UiError::host("CreateDIBSection returned null pixels"))
                }
                Ok(()) => Ok(bgra_to_rgba(bits, w, h, scale_factor, backend)),
            }
        };
        // SAFETY: `bitmap` is our live GDI object, unselected above.
        unsafe { DeleteObject(bitmap as HGDIOBJ) };
        result
    }

    /// Copies `w*h` BGRA pixels at `bits` into RGBA8 with alpha
    /// forced opaque. Row stride is `w * 4` — always 4-byte aligned,
    /// so DIB padding cannot occur.
    ///
    /// # Safety
    /// `bits` must point to at least `w * h * 4` readable bytes.
    unsafe fn bgra_to_rgba(
        bits: *mut c_void,
        w: u32,
        h: u32,
        scale_factor: f64,
        backend: &'static str,
    ) -> OsCapturedFrame {
        let count = usize::try_from(w)
            .and_then(|w| usize::try_from(h).map(|h| w * h))
            .expect("client area pixel count fits usize");
        let src = unsafe { std::slice::from_raw_parts(bits.cast::<u8>(), count * 4) };
        let mut pixels = Vec::with_capacity(count * 4);
        for px in src.as_chunks::<4>().0 {
            // DIB 32bpp BI_RGB memory order is B,G,R,X.
            pixels.extend_from_slice(&[px[2], px[1], px[0], 0xFF]);
        }
        OsCapturedFrame {
            width: w,
            height: h,
            scale_factor,
            pixels,
            backend,
        }
    }

    /// `GetLastError` as a [`UiError::Host`] with the failing call's
    /// name — capture failures are host errors, not verdicts.
    fn last_error(call: &str) -> UiError {
        // SAFETY: `GetLastError` reads the calling thread's error
        // value; no preconditions.
        let code = unsafe { GetLastError() };
        UiError::host(format!("{call} failed (win32 error {code})"))
    }
}

/// Linux X11 backend — real window pixel readback through the X
/// protocol (`GetImage` on the window drawable), Issue #314
/// RF-GT-05 offscreen lane.
///
/// `raw_parent` is [`X11_RESOLVE_BY_PID`] — gpui 0.2.2's X11
/// `PlatformWindow` does not implement `HasWindowHandle`, so the
/// driver records the sentinel at open and this backend resolves
/// the window id lazily through `_NET_WM_PID` (see [`own_window`]).
/// A fresh `x11rb` connection can query and read the window gpui
/// created on its own connection — X resources are server-global.
///
/// Pixels come only from the X server's own rendered window
/// contents — never semantic state, draw plans, or observer output.
/// Under `xvfb-run` (the CI offscreen lane) the "screen" is a
/// virtual framebuffer with no GPU or physical display, so the
/// readback is deterministic; on a real X display it measures the
/// displayed window, including border pixels when a WM sets
/// `border_width` (Xvfb runs no WM, so border is 0 there).
///
/// The pixel layout is resolved per capture: `x11rb::image::Image`
/// normalizes scanline padding, bits-per-pixel, and byte order, and
/// `PixelLayout::from_visual_type` decodes R/G/B through the
/// window's own visual masks. Palette/gray-scale visuals cannot be
/// decoded honestly and return [`UiError::Unsupported`].
#[cfg(target_os = "linux")]
mod backend {
    use ruviform_ui::UiError;
    use x11rb::connection::Connection;
    use x11rb::image::{Image, PixelLayout};
    use x11rb::protocol::xproto::{
        AtomEnum, ConnectionExt, Drawable, GetGeometryReply, Setup, Visualtype, Window,
    };
    use x11rb::rust_connection::RustConnection;

    use super::{OsCapturedFrame, OsWindowGeometry};

    /// Connects to the X server named by `DISPLAY` — `None` lets
    /// x11rb apply its standard resolution (env var, then the local
    /// socket). A headless host without X fails here, which the
    /// scenario layer records as `environment_blocked` upstream.
    fn connect() -> Result<(RustConnection, usize), UiError> {
        x11rb::connect(None)
            .map_err(|e| UiError::host(format!("X11 connect failed — is DISPLAY set? ({e})")))
    }

    /// `raw_parent` → X11 window id. [`X11_RESOLVE_BY_PID`] resolves
    /// this process's own toplevel through `_NET_WM_PID`; any other
    /// value is treated as a literal XID (a diagnostic path —
    /// `raw_parent_of` never produces one on Linux today). Values
    /// that cannot be a XID are host errors, not silent truncation.
    fn window_xid(
        conn: &RustConnection,
        screen: usize,
        raw_parent: usize,
    ) -> Result<Window, UiError> {
        if raw_parent == super::X11_RESOLVE_BY_PID {
            return own_window(conn, screen);
        }
        u32::try_from(raw_parent)
            .map_err(|_| UiError::host("window handle does not fit an X11 window id"))
    }

    /// The single X11 toplevel owned by this process. gpui sets
    /// `_NET_WM_PID` to `std::process::id()` at open
    /// (`platform/linux/x11/window.rs`), so the property identifies
    /// our windows. Candidates come from the EWMH `_NET_CLIENT_LIST`
    /// when a window manager maintains it — client windows are
    /// reparented into frame windows under a WM, so root children
    /// alone are not reliable — and fall back to a full `QueryTree`
    /// walk for WM-less servers such as bare Xvfb. Zero or multiple
    /// pid matches are host errors: guessing which window to read
    /// would fabricate evidence.
    fn own_window(conn: &RustConnection, screen: usize) -> Result<Window, UiError> {
        let pid_atom = conn
            .intern_atom(false, b"_NET_WM_PID")
            .map_err(|e| UiError::host(format!("InternAtom _NET_WM_PID failed: {e}")))?
            .reply()
            .map_err(|e| UiError::host(format!("InternAtom _NET_WM_PID reply failed: {e}")))?
            .atom;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .map(|root| root.root)
            .ok_or_else(|| UiError::host("X11 screen has no root window"))?;
        let candidates = client_list(conn, root)?.unwrap_or_else(|| tree_windows(conn, root));
        // x11rb negotiates the connection in host byte order, so a
        // format-32 property unit compares directly against the
        // native-endian pid bytes.
        let pid_bytes = std::process::id().to_ne_bytes();
        let mut found = Vec::new();
        for child in candidates {
            let property = conn
                .get_property(false, child, pid_atom, AtomEnum::CARDINAL, 0, 1)
                .map_err(|e| UiError::host(format!("GetProperty _NET_WM_PID failed: {e}")))?
                .reply()
                .map_err(|e| UiError::host(format!("GetProperty _NET_WM_PID reply failed: {e}")))?;
            if property.format == 32
                && property.value.len() >= 4
                && property.value[..4] == pid_bytes
            {
                found.push(child);
            }
        }
        match found.as_slice() {
            [xid] => Ok(*xid),
            [] => Err(UiError::host(
                "no X11 toplevel carries this process's _NET_WM_PID",
            )),
            _ => Err(UiError::host(format!(
                "{} X11 toplevels carry this process's _NET_WM_PID — window identity is ambiguous",
                found.len()
            ))),
        }
    }

    /// `_NET_CLIENT_LIST` on the root — the EWMH-managed set of
    /// client toplevels. `None` when no window manager maintains the
    /// property (bare Xvfb), which selects the tree-walk fallback.
    /// A present-but-malformed property is a host error, not a
    /// silent fallback: the WM claimed to manage the list.
    fn client_list(conn: &RustConnection, root: Window) -> Result<Option<Vec<Window>>, UiError> {
        let atom = conn
            .intern_atom(false, b"_NET_CLIENT_LIST")
            .map_err(|e| UiError::host(format!("InternAtom _NET_CLIENT_LIST failed: {e}")))?
            .reply()
            .map_err(|e| UiError::host(format!("InternAtom _NET_CLIENT_LIST reply failed: {e}")))?
            .atom;
        let property = conn
            .get_property(false, root, atom, AtomEnum::WINDOW, 0, u32::MAX)
            .map_err(|e| UiError::host(format!("GetProperty _NET_CLIENT_LIST failed: {e}")))?
            .reply()
            .map_err(|e| {
                UiError::host(format!("GetProperty _NET_CLIENT_LIST reply failed: {e}"))
            })?;
        if property.type_ == 0 {
            return Ok(None);
        }
        if property.format != 32 || property.value.len() % 4 != 0 {
            return Err(UiError::host(
                "_NET_CLIENT_LIST has an unexpected format — cannot enumerate client windows",
            ));
        }
        // `len % 4 == 0` was checked above, so the remainder is empty.
        let (units, _) = property.value.as_chunks::<4>();
        let windows = units.iter().map(|unit| u32::from_ne_bytes(*unit)).collect();
        Ok(Some(windows))
    }

    /// Every window in the server's window tree below `root`,
    /// collected by recursive `QueryTree`. Used only when no window
    /// manager maintains `_NET_CLIENT_LIST`; the pid filter in
    /// [`own_window`] keeps frame/helper windows out of the match.
    fn tree_windows(conn: &RustConnection, root: Window) -> Vec<Window> {
        fn visit(conn: &RustConnection, window: Window, depth: u32, out: &mut Vec<Window>) {
            // Bound the walk: X server trees are shallow, and a
            // pathological server must not turn capture into an
            // unbounded request loop.
            if depth > 16 || out.len() > 4096 {
                return;
            }
            let Ok(cookie) = conn.query_tree(window) else {
                return;
            };
            let Ok(reply) = cookie.reply() else {
                return;
            };
            for child in reply.children {
                out.push(child);
                visit(conn, child, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        visit(conn, root, 0, &mut out);
        out
    }

    /// `GetGeometry` on the window drawable — position is
    /// parent-relative; `client_geometry` translates it to root
    /// coordinates for input targeting.
    fn geometry(conn: &RustConnection, xid: Window) -> Result<GetGeometryReply, UiError> {
        conn.get_geometry(xid as Drawable)
            .map_err(|e| UiError::host(format!("GetGeometry failed: {e}")))?
            .reply()
            .map_err(|e| UiError::host(format!("GetGeometry reply failed: {e}")))
    }

    /// The `Visualtype` matching `visual_id` in the selected screen's
    /// setup — supplies the R/G/B masks for pixel decode.
    fn find_visual(setup: &Setup, screen: usize, visual_id: u32) -> Result<Visualtype, UiError> {
        setup
            .roots
            .get(screen)
            .into_iter()
            .flat_map(|root| root.allowed_depths.iter())
            .flat_map(|depth| depth.visuals.iter())
            .find(|visual| visual.visual_id == visual_id)
            .copied()
            .ok_or_else(|| UiError::host("window visual not found in server setup"))
    }

    /// Captures the window drawable's pixels via `GetImage`
    /// (`ZPixmap`, all planes) and decodes them through the window's
    /// visual.
    ///
    /// # Errors
    /// [`UiError::Host`] on connect/geometry/readback failure or an
    /// empty window; [`UiError::Unsupported`] when the window's
    /// visual is not TrueColor/DirectColor (palette and gray-scale
    /// visuals have no honest RGB decode in this slice).
    pub(super) fn capture_client_area(
        raw_parent: usize,
        scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        let (conn, screen) = connect()?;
        let xid = window_xid(&conn, screen, raw_parent)?;
        let geo = geometry(&conn, xid)?;
        if geo.width == 0 || geo.height == 0 {
            return Err(UiError::host("window has an empty geometry"));
        }
        let (image, visual_id) = Image::get(&conn, xid as Drawable, 0, 0, geo.width, geo.height)
            .map_err(|e| UiError::host(format!("GetImage failed: {e}")))?;
        let visual = find_visual(conn.setup(), screen, visual_id)?;
        let layout = PixelLayout::from_visual_type(visual).map_err(|_| {
            UiError::unsupported(
                "visual_capture",
                "window visual is not TrueColor/DirectColor — no honest RGB decode",
            )
        })?;
        let (w, h) = (image.width(), image.height());
        let mut pixels = Vec::with_capacity(usize::from(w) * usize::from(h) * 4);
        for row in 0..h {
            for col in 0..w {
                let (red, green, blue) = layout.decode(image.get_pixel(col, row));
                // `decode` expands each component to 16 bits; the
                // top byte is the 8-bit channel value.
                pixels.extend_from_slice(&[
                    (red >> 8) as u8,
                    (green >> 8) as u8,
                    (blue >> 8) as u8,
                    0xFF,
                ]);
            }
        }
        Ok(OsCapturedFrame {
            width: u32::from(w),
            height: u32::from(h),
            scale_factor,
            pixels,
            backend: "x11-getimage",
        })
    }

    /// Client-area geometry of the window drawable in root
    /// coordinates — the input-targeting query. `GetGeometry` alone
    /// reports parent-relative coordinates, which are wrong for a
    /// WM-reparented toplevel, so the origin is translated to the
    /// root window; the returned size is the drawable's inside size,
    /// which already excludes any `border_width`.
    ///
    /// # Errors
    /// [`UiError::Host`] on connect/geometry/translate failure or an
    /// empty client area.
    pub(super) fn client_geometry(
        raw_parent: usize,
        scale_factor: f64,
    ) -> Result<OsWindowGeometry, UiError> {
        let (conn, screen) = connect()?;
        let xid = window_xid(&conn, screen, raw_parent)?;
        let geo = geometry(&conn, xid)?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .map(|root| root.root)
            .ok_or_else(|| UiError::host("X11 screen has no root window"))?;
        let origin = conn
            .translate_coordinates(xid, root, 0, 0)
            .map_err(|e| UiError::host(format!("TranslateCoordinates failed: {e}")))?
            .reply()
            .map_err(|e| UiError::host(format!("TranslateCoordinates reply failed: {e}")))?;
        // `GetGeometry` width/height is the window's inside size —
        // the border is outside the drawable, so the client area is
        // exactly geo.width x geo.height and `border_width` plays no
        // part. `TranslateCoordinates(0,0)` lands on the drawable's
        // top-left, inside any border.
        let w = i64::from(geo.width);
        let h = i64::from(geo.height);
        if w <= 0 || h <= 0 {
            return Err(UiError::host("window client area is empty"));
        }
        Ok(OsWindowGeometry {
            origin_x: i32::from(origin.dst_x),
            origin_y: i32::from(origin.dst_y),
            width: u32::try_from(w).map_err(|_| UiError::host("client width overflows u32"))?,
            height: u32::try_from(h).map_err(|_| UiError::host("client height overflows u32"))?,
            scale_factor,
        })
    }
}

/// Remaining non-Windows/non-Linux placeholder (macOS and other
/// targets) — the `raw_parent` token is an NSView handle this slice
/// does not capture or measure. Honest [`UiError::Unsupported`],
/// never fabricated geometry.
#[cfg(not(any(windows, target_os = "linux")))]
mod backend {
    use ruviform_ui::UiError;

    use super::{OsCapturedFrame, OsWindowGeometry};

    /// Always unsupported on this target in this slice.
    pub(super) fn capture_client_area(
        _raw_parent: usize,
        _scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        Err(UiError::unsupported(
            "visual_capture",
            "OS window capture is implemented for Windows and Linux (X11) only in this slice",
        ))
    }

    /// Always unsupported on this target in this slice.
    pub(super) fn client_geometry(
        _raw_parent: usize,
        _scale_factor: f64,
    ) -> Result<OsWindowGeometry, UiError> {
        Err(UiError::unsupported(
            "window_geometry",
            "OS window geometry is implemented for Windows and Linux (X11) only in this slice",
        ))
    }
}
