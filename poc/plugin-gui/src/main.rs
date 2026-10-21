//! Issue #30 PoC — 外部親 HWND（素 Win32 窓 + 素 GetMessage ループ）への
//! wry WebView 子 attach。プラグイン GUI がホストから受け取る状況の模倣。
//!
//! 計測は `poc-metric.v1` 形式（JSON Lines）を stdout と
//! `--out <path>` のファイルへ出力する。

use std::cell::Cell;
use std::fs::File;
use std::io::Write;
use std::num::NonZeroIsize;
use std::rc::Rc;
use std::time::Instant;

use dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use raw_window_handle::{
    HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};

/// raw-window-handle adapter around a host-provided HWND — the same
/// shape a plugin receives via `clap_window.hwnd` / VST3 `IPlugView`.
struct HostWindow(windows::Win32::Foundation::HWND);
impl HasWindowHandle for HostWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw = NonZeroIsize::new(self.0.0 as isize)
            .map(|h| RawWindowHandle::Win32(Win32WindowHandle::new(h)))
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

struct Metrics {
    out: Option<File>,
}
impl Metrics {
    fn emit(&mut self, metric: &str, result: &str, detail: &str) {
        let line = serde_json::json!({
            "schema": "poc-metric.v1",
            "metric": metric,
            "result": result,
            "detail": detail,
        });
        let s = line.to_string();
        println!("{s}");
        if let Some(f) = &mut self.out {
            let _ = writeln!(f, "{s}");
        }
    }
}

fn working_set_bytes() -> u64 {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut c = PROCESS_MEMORY_COUNTERS::default();
    // SAFETY: GetCurrentProcess は常に有効な擬似ハンドル。カウンタ構造体は
    // 正しいサイズで初期化済み。
    unsafe {
        let _ = GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut c,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
    }
    c.WorkingSetSize as u64
}

#[cfg(windows)]
unsafe extern "system" fn host_wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    // SAFETY: 素 Win32 窓プロシージャ — DefWindowProcW へ全転送。
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(windows)]
fn run() -> i32 {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    let out = std::env::args()
        .position(|a| a == "--out")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|p| File::create(p).ok());
    let mut m = Metrics { out };

    let mem_before = working_set_bytes();

    // 1. 素 Win32 親窓（ホスト提供ウィンドウの模倣 — tao 不使用）。
    // SAFETY: 呼出しは Win32 の標準契約。class 登録・hwnd・wndproc は
    // 本スレッド内で完結し、ポインタの有効性は Win32 が保証する。
    let hwnd = unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let class = w!("plugin_gui_poc_parent");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(host_wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("plugin-gui-poc host parent"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            600,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };
    let parent = HostWindow(hwnd);

    // 2. WebView を外部親へ子として attach（AC1 の核心操作）。
    let ipc_got = Rc::new(Cell::new(false));
    let loaded = Rc::new(Cell::new(false));
    let wv_result = {
        let ipc_got = Rc::clone(&ipc_got);
        let loaded = Rc::clone(&loaded);
        WebViewBuilder::new()
            .with_html(
                r#"<!doctype html><title>plugin-gui-poc</title>
                <body style="background:#224">embedded</body>
                <script>window.ipc.postMessage('ping')</script>"#,
            )
            .with_ipc_handler(move |req| {
                if req.body() == "ping" {
                    ipc_got.set(true);
                }
            })
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, PageLoadEvent::Finished) {
                    loaded.set(true);
                }
            })
            .with_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(PhysicalSize::new(800, 600)),
            })
            .build_as_child(&parent)
    };
    let webview: WebView = match wv_result {
        Ok(w) => {
            m.emit("e1_attach", "passed", "build_as_child on raw host HWND");
            w
        }
        Err(e) => {
            m.emit("e1_attach", "failed", &format!("{e}"));
            return 1;
        }
    };

    // 3. 素 GetMessage ループ（ホスト所有のイベントループの模倣）。
    //    WM_TIMER で期限を管理し、実験完了か 20s で終了。
    let start = Instant::now();
    let mut resize_done = false;
    let mut ipc_checked = false;
    unsafe {
        SetTimer(Some(hwnd), 1, 50, None);
        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return 0;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if loaded.get() && !resize_done {
                resize_done = true;
                let ok = webview.set_bounds(Rect {
                    position: Position::Physical(PhysicalPosition::new(10, 10)),
                    size: Size::Physical(PhysicalSize::new(600, 400)),
                });
                m.emit(
                    "e1_resize",
                    if ok.is_ok() { "passed" } else { "failed" },
                    "set_bounds 600x400 after attach",
                );
                m.emit("e1_render", "passed", "PageLoadEvent::Finished");
            }
            if ipc_got.get() && !ipc_checked {
                ipc_checked = true;
                m.emit("e1_ipc", "passed", "window.ipc.postMessage ping received");
            }
            // 全実験完了（load → resize → IPC）後にまとめて終了
            if loaded.get() && resize_done && ipc_checked {
                m.emit(
                    "e2_mem_delta_bytes",
                    "passed",
                    &(working_set_bytes() as i64 - mem_before as i64).to_string(),
                );
                m.emit(
                    "e2_elapsed_ms",
                    "passed",
                    &start.elapsed().as_millis().to_string(),
                );
                let _ = PostMessageW(Some(hwnd), WM_QUIT, WPARAM(0), LPARAM(0));
            }
            if start.elapsed().as_secs() > 20 {
                m.emit("e1_ipc", "failed", "timeout waiting for ipc");
                return 1;
            }
            let _ = MsgWaitForMultipleObjects(None, false, 100, QS_ALLINPUT);
        }
    }
}

#[cfg(not(windows))]
fn run() -> i32 {
    eprintln!("plugin-gui-poc: windows only (this host is skipped)");
    2
}

fn main() {
    std::process::exit(run());
}
