//! OS pixel observation for the real-window dock lane (Issue #532
//! dcomp-pixel slice — feature `real-dock`).
//!
//! Query/capture-only Win32 pixel reads used by the
//! `real-dcomp-pixel` case — the same boundary convention as the
//! sibling `oschild` module inside [`crate::real_dock`]: this module
//! never creates, destroys, moves, or messages a window; it only
//! reads rectangles and copies pixels out of device contexts.
//!
//! One primary oracle and two deliberately distinct diagnostics:
//!
//! - [`capture_screen_rect`] — `BitBlt` from the screen DC over a
//!   window's *screen* rectangle: the final composited result the
//!   user sees (`DComp` visuals, other HWNDs, and the cursor
//!   included). This is the primary oracle for the Issue #532
//!   topmost-`DComp` occlusion question.
//! - [`capture_print_window`] — `PrintWindow` on the child HWND:
//!   the child's *own* raster, not the on-screen composition. A
//!   diagnostic axis only — it can show the plugin painted its
//!   surface even when the screen never displays it.
//! - [`capture_window_dc`] — `GetDC` + `BitBlt` of the child HWND's
//!   client area: the window's own backing surface as GDI last
//!   painted it, bypassing both screen composition and
//!   `PrintWindow`'s `WM_PRINT` path (which plain `DefWindowProcW`
//!   classes may not honour). The second diagnostic axis — it
//!   separates "child painted green but the compositor hides it"
//!   from "the child surface itself never received the fill".
//!
//! All values crossing the module boundary are plain integers and
//! RGBA8 bytes — no Win32 type reaches the caller (the same
//! `u64`-narrowed-handle convention as `oschild`).
//!
//! **RT contract:** non-RT only. **Test-only:** nothing here ships.

/// A window rectangle in virtual-screen coordinates (physical
/// pixels — `GetWindowRect` space, so multi-monitor negative
/// origins are preserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScreenRect {
    /// Left edge, virtual-screen x.
    pub left: i32,
    /// Top edge, virtual-screen y.
    pub top: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

/// One captured frame — driver-neutral data only. `rgba` is
/// row-major RGBA8, top-down, alpha forced opaque.
#[derive(Debug, Clone)]
pub(crate) struct PixelCapture {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Row-major RGBA8 pixels (`len == width * height * 4`).
    pub rgba: Vec<u8>,
    /// Which OS path produced the pixels — `win32-bitblt-screen`,
    /// `win32-printwindow`, or `win32-windowdc` — recorded into
    /// step rows as evidence.
    pub backend: &'static str,
}

/// `GetWindowRect(hwnd)` mapped to a [`ScreenRect`].
///
/// # Errors
/// A string naming the failing call when the query fails or the
/// window reports an empty rectangle.
pub(crate) fn window_screen_rect(hwnd: u64) -> Result<ScreenRect, String> {
    platform::window_screen_rect(hwnd)
}

/// Whether the window's top-level is presented — visible and not
/// minimized — so screen pixels are meaningful. `Some(false)` covers
/// iconic/hidden (the honest `not_run` input); `None` means the
/// query itself failed or the platform has no such concept (the
/// non-Windows stub).
pub(crate) fn presented(hwnd: u64) -> Option<bool> {
    platform::presented(hwnd)
}

/// `BitBlt`s the screen rectangle into a fresh top-down 32bpp DIB —
/// the composited result the user actually sees.
///
/// # Errors
/// A string naming the failing GDI call.
pub(crate) fn capture_screen_rect(rect: ScreenRect) -> Result<PixelCapture, String> {
    platform::capture_screen_rect(rect)
}

/// `PrintWindow`s the child HWND's client area into a fresh
/// top-down 32bpp DIB — the child's own raster, NOT the on-screen
/// composition.
///
/// # Errors
/// A string naming the failing call (dead handle, `PrintWindow`
/// refusal, GDI failure).
pub(crate) fn capture_print_window(hwnd: u64) -> Result<PixelCapture, String> {
    platform::capture_print_window(hwnd)
}

/// `BitBlt`s the child HWND's client area out of its own window DC
/// into a fresh top-down 32bpp DIB — the surface GDI last painted,
/// read without involving screen composition or `WM_PRINT`.
///
/// # Errors
/// A string naming the failing call (dead handle, GDI failure).
pub(crate) fn capture_window_dc(hwnd: u64) -> Result<PixelCapture, String> {
    platform::capture_window_dc(hwnd)
}

#[cfg(windows)]
#[expect(unsafe_code)]
mod platform {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HDC, HGDIOBJ, ReleaseDC, SRCCOPY,
        SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GetAncestor, GetClientRect, GetWindowRect, IsIconic, IsWindowVisible,
    };

    use super::{PixelCapture, ScreenRect};

    /// `PrintWindow` `nflags`: client area only, full rendered
    /// content (`PW_CLIENTONLY | PW_RENDERFULLCONTENT`). The Win32
    /// ABI values are stable; windows-sys gates `PW_CLIENTONLY`
    /// behind the disabled `Win32_Storage_Xps` feature — same local
    /// declaration as `ruviform-ui-gpui`'s capture backend.
    const PRINT_WINDOW_CLIENT_FULL: u32 = 0x1 | 0x2;

    #[link(name = "user32")]
    unsafe extern "system" {
        /// `PrintWindow` — see the const above for why this is a
        /// local declaration rather than a windows-sys import.
        fn PrintWindow(hwnd: HWND, hdcblt: HDC, nflags: u32) -> i32;
    }

    /// Screen-DC handle released on drop — keeps the GDI cleanup
    /// paired with `GetDC` on every exit path.
    struct ScreenDc(HDC);

    impl Drop for ScreenDc {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a live screen DC from `GetDC`;
            // `ReleaseDC` with a null window releases it exactly
            // once.
            unsafe { ReleaseDC(ptr::null_mut(), self.0) };
        }
    }

    /// `GetWindowRect` on `hwnd`, mapped to [`ScreenRect`].
    pub(super) fn window_screen_rect(hwnd: u64) -> Result<ScreenRect, String> {
        let mut rect = RECT::default();
        // SAFETY: `hwnd` is an opaque handle the caller observed
        // alive moments ago; `rect` is a valid out-param.
        if unsafe { GetWindowRect(hwnd as HWND, &raw mut rect) } == 0 {
            return Err(last_error("GetWindowRect"));
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err(format!("window rect is empty ({width}x{height})"));
        }
        Ok(ScreenRect {
            left: rect.left,
            top: rect.top,
            width: u32::try_from(width)
                .map_err(|_| "window rect width overflows u32".to_string())?,
            height: u32::try_from(height)
                .map_err(|_| "window rect height overflows u32".to_string())?,
        })
    }

    /// Top-level presentation state of `hwnd`'s root — `GA_ROOT`
    /// walks past child windows to the top-level owner.
    pub(super) fn presented(hwnd: u64) -> Option<bool> {
        // SAFETY: read-only ancestor walk + presentation queries on
        // an opaque handle.
        unsafe {
            let root = GetAncestor(hwnd as HWND, GA_ROOT);
            if root.is_null() {
                return None;
            }
            Some(IsIconic(root) == 0 && IsWindowVisible(root) != 0)
        }
    }

    /// `BitBlt`s `rect` (virtual-screen coordinates) into a fresh
    /// top-down 32bpp DIB — the composited result the user sees.
    pub(super) fn capture_screen_rect(rect: ScreenRect) -> Result<PixelCapture, String> {
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
                rect.width,
                rect.height,
                "win32-bitblt-screen",
                |dc| {
                    // SAFETY: `dc` is a live memory DC with our DIB
                    // selected; `screen` is a live screen DC; the src
                    // rect is `rect` on the virtual screen.
                    if BitBlt(
                        dc,
                        0,
                        0,
                        i32::try_from(rect.width).expect("rect width fits i32"),
                        i32::try_from(rect.height).expect("rect height fits i32"),
                        screen.0,
                        rect.left,
                        rect.top,
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

    /// `PrintWindow`s `hwnd`'s client area into a fresh top-down
    /// 32bpp DIB sized to `GetClientRect` — the child's own raster.
    pub(super) fn capture_print_window(hwnd: u64) -> Result<PixelCapture, String> {
        let mut client = RECT::default();
        // SAFETY: `client` is a valid out-param; `hwnd` is an opaque
        // handle the caller observed alive moments ago.
        if unsafe { GetClientRect(hwnd as HWND, &raw mut client) } == 0 {
            return Err(last_error("GetClientRect"));
        }
        let w = client.right - client.left;
        let h = client.bottom - client.top;
        if w <= 0 || h <= 0 {
            return Err(format!("client rect is empty ({w}x{h})"));
        }
        let width = u32::try_from(w).map_err(|_| "client width overflows u32".to_string())?;
        let height = u32::try_from(h).map_err(|_| "client height overflows u32".to_string())?;

        // SAFETY: null HWND requests the entire screen DC — it only
        // supplies the pixel format for the memory DC/DIB.
        let screen = unsafe { GetDC(ptr::null_mut()) };
        if screen.is_null() {
            return Err(last_error("GetDC"));
        }
        let screen = ScreenDc(screen);

        // SAFETY: `screen.0` is a live screen DC; `hwnd` is the
        // observed-live handle; every GDI object below is created,
        // used, and destroyed within this scope.
        unsafe {
            let mem = CreateCompatibleDC(screen.0);
            if mem.is_null() {
                return Err(last_error("CreateCompatibleDC"));
            }
            let result = dib_capture(mem, screen.0, width, height, "win32-printwindow", |dc| {
                // SAFETY: `dc` is a live memory DC with our DIB
                // selected; `hwnd` is the observed-live handle;
                // the flags request the client area rendered
                // with full content.
                if PrintWindow(hwnd as HWND, dc, PRINT_WINDOW_CLIENT_FULL) != 0 {
                    Ok(())
                } else {
                    Err(last_error("PrintWindow"))
                }
            });
            // SAFETY: `mem` is our live compatible DC.
            DeleteDC(mem);
            result
        }
    }

    /// `GetDC`s `hwnd` and `BitBlt`s its client area into a fresh
    /// top-down 32bpp DIB — the window's own backing surface as GDI
    /// last painted it. For a child HWND this reads its private
    /// surface directly, so a green class-brush fill shows up even
    /// when a `DComp` visual covers it on screen.
    pub(super) fn capture_window_dc(hwnd: u64) -> Result<PixelCapture, String> {
        let mut client = RECT::default();
        // SAFETY: `client` is a valid out-param; `hwnd` is an opaque
        // handle the caller observed alive moments ago.
        if unsafe { GetClientRect(hwnd as HWND, &raw mut client) } == 0 {
            return Err(last_error("GetClientRect"));
        }
        let w = client.right - client.left;
        let h = client.bottom - client.top;
        if w <= 0 || h <= 0 {
            return Err(format!("client rect is empty ({w}x{h})"));
        }
        let width = u32::try_from(w).map_err(|_| "client width overflows u32".to_string())?;
        let height = u32::try_from(h).map_err(|_| "client height overflows u32".to_string())?;

        // SAFETY: `hwnd` is the observed-live handle; the returned
        // DC is released by `WindowDc` on every exit path.
        let window_dc = unsafe { GetDC(hwnd as HWND) };
        if window_dc.is_null() {
            return Err(last_error("GetDC(hwnd)"));
        }
        let window_dc = WindowDc {
            hwnd: hwnd as HWND,
            dc: window_dc,
        };

        // SAFETY: null HWND requests the entire screen DC — it only
        // supplies the pixel format for the memory DC/DIB.
        let screen = unsafe { GetDC(ptr::null_mut()) };
        if screen.is_null() {
            return Err(last_error("GetDC"));
        }
        let screen = ScreenDc(screen);

        // SAFETY: `screen.0` is a live screen DC; `window_dc.0` is a
        // live window DC; every GDI object below is created, used,
        // and destroyed within this scope.
        unsafe {
            let mem = CreateCompatibleDC(screen.0);
            if mem.is_null() {
                return Err(last_error("CreateCompatibleDC"));
            }
            let result = dib_capture(mem, screen.0, width, height, "win32-windowdc", |dc| {
                // SAFETY: `dc` is a live memory DC with our DIB
                // selected; `window_dc` is a live window DC whose
                // origin is the client area's top-left for a
                // non-overlapped child window.
                if BitBlt(
                    dc,
                    0,
                    0,
                    i32::try_from(width).expect("client width fits i32"),
                    i32::try_from(height).expect("client height fits i32"),
                    window_dc.dc,
                    0,
                    0,
                    SRCCOPY,
                ) != 0
                {
                    Ok(())
                } else {
                    Err(last_error("BitBlt"))
                }
            });
            // SAFETY: `mem` is our live compatible DC.
            DeleteDC(mem);
            result
        }
    }

    /// Window DC released on drop — keeps `GetDC(hwnd)`/`ReleaseDC`
    /// paired with the owning window on every exit path.
    struct WindowDc {
        hwnd: HWND,
        dc: HDC,
    }

    impl Drop for WindowDc {
        fn drop(&mut self) {
            // SAFETY: `self.dc` is a live window DC from `GetDC` on
            // `self.hwnd`; `ReleaseDC` pairs it exactly once.
            unsafe { ReleaseDC(self.hwnd, self.dc) };
        }
    }

    /// Creates the DIB, runs `render` against the selected memory DC,
    /// converts BGRA → RGBA8, and destroys the bitmap — factored out
    /// so both capture paths share the DIB plumbing (the
    /// `gpui_driver/capture.rs` backend pattern, narrowed to this
    /// lane's needs).
    ///
    /// # Safety
    /// `mem`/`ref_dc` must be live DCs; `render` must only draw into
    /// the memory DC it is given; all GDI objects are local.
    unsafe fn dib_capture(
        mem: HDC,
        ref_dc: HDC,
        w: u32,
        h: u32,
        backend: &'static str,
        render: impl FnOnce(HDC) -> Result<(), String>,
    ) -> Result<PixelCapture, String> {
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize =
            u32::try_from(size_of::<BITMAPINFOHEADER>()).expect("BITMAPINFOHEADER size fits u32");
        bmi.bmiHeader.biWidth = i32::try_from(w).expect("capture width fits i32");
        // Negative height requests a top-down DIB — rows arrive in
        // display order without a flip pass.
        bmi.bmiHeader.biHeight = -i32::try_from(h).expect("capture height fits i32");
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
                    Err("CreateDIBSection returned null pixels".to_string())
                }
                Ok(()) => Ok(bgra_to_rgba(bits, w, h, backend)),
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
        backend: &'static str,
    ) -> PixelCapture {
        let count = usize::try_from(w)
            .and_then(|w| usize::try_from(h).map(|h| w * h))
            .expect("capture pixel count fits usize");
        // SAFETY: `bits` points to the DIB section's `w*h*4` bytes.
        let src = unsafe { std::slice::from_raw_parts(bits.cast::<u8>(), count * 4) };
        let mut rgba = Vec::with_capacity(count * 4);
        for px in src.as_chunks::<4>().0 {
            // DIB 32bpp BI_RGB memory order is B,G,R,X.
            rgba.extend_from_slice(&[px[2], px[1], px[0], 0xFF]);
        }
        PixelCapture {
            width: w,
            height: h,
            rgba,
            backend,
        }
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
/// unreachable in practice; it exists so the crate compiles on every
/// target and an accepted open would record honest errors/`None`,
/// never fabricated pixels.
#[cfg(not(windows))]
mod platform {
    use super::{PixelCapture, ScreenRect};

    pub(super) fn window_screen_rect(_hwnd: u64) -> Result<ScreenRect, String> {
        Err("OS pixel observation is Windows-only".to_string())
    }

    pub(super) fn presented(_hwnd: u64) -> Option<bool> {
        None
    }

    pub(super) fn capture_screen_rect(_rect: ScreenRect) -> Result<PixelCapture, String> {
        Err("OS pixel observation is Windows-only".to_string())
    }

    pub(super) fn capture_print_window(_hwnd: u64) -> Result<PixelCapture, String> {
        Err("OS pixel observation is Windows-only".to_string())
    }

    pub(super) fn capture_window_dc(_hwnd: u64) -> Result<PixelCapture, String> {
        Err("OS pixel observation is Windows-only".to_string())
    }
}
