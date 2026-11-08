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
    /// desktop content such as the cursor).
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
    ///   in this slice (non-Windows).
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

    use super::OsCapturedFrame;

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

/// Non-Windows placeholder — the `raw_parent` token is an
/// NSView/X11 handle this slice does not capture. Honest
/// [`UiError::Unsupported`], never fabricated pixels.
#[cfg(not(windows))]
mod backend {
    use ruviform_ui::UiError;

    use super::OsCapturedFrame;

    /// Always unsupported on non-Windows targets in this slice.
    pub(super) fn capture_client_area(
        _raw_parent: usize,
        _scale_factor: f64,
    ) -> Result<OsCapturedFrame, UiError> {
        Err(UiError::unsupported(
            "visual_capture",
            "OS window capture is implemented for Windows only in this slice",
        ))
    }
}
