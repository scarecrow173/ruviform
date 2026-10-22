//! Issue #30 PoC — 外部親 HWND（素 Win32 窓 + 素 GetMessage ループ）への
//! wry WebView 子 attach。プラグイン GUI がホストから受け取る状況の模倣。
//!
//! 計測は `poc-metric.v1` 形式（JSON Lines）を stdout と
//! `--out <path>` のファイルへ出力する。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
#[cfg(windows)]
use std::num::NonZeroIsize;
use std::rc::Rc;
use std::time::Instant;

use dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};

#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};

/// raw-window-handle adapter around a host-provided HWND — the same
/// shape a plugin receives via `clap_window.hwnd` / VST3 `IPlugView`.
#[cfg(windows)]
struct HostWindow(windows::Win32::Foundation::HWND);
#[cfg(windows)]
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

#[cfg(windows)]
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

#[cfg(target_os = "linux")]
fn working_set_bytes() -> u64 {
    // /proc/self/status の VmRSS（working set 相当、KiB 表記）。
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn working_set_bytes() -> u64 {
    0
}

#[cfg(windows)]
unsafe extern "system" fn host_wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    // SAFETY: 素 Win32 窓プロシージャ — DefWindowProcW へ全転送。
    unsafe { windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(windows)]
fn run() -> i32 {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::w;

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

/// Issue #57 高負荷再計測 — 複数プラグイン GUI 共存の近似実験。
/// `--count N --frames M` で N 個の子 HWND にそれぞれ wry WebView を
/// `build_as_child` で attach し、各 WebView がプラグイン GUI 相当の
/// rAF アニメーション（回転ノブ 24 個）を M フレーム回したのち
/// `window.ipc.postMessage` で統計を返す。全 WebView 完了時に
/// 集約メトリクスを poc-metric.v1 で出力する。
#[cfg(windows)]
fn run_multi(count: usize, frames: u32, duration_secs: u64, out: Option<File>) -> i32 {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::w;

    let mut m = Metrics { out };
    let mem_before = working_set_bytes();

    // グリッド配置（セル 400x300。count 上限は画面実用域として 16）。
    let cols = (count as f64).sqrt().ceil() as u32;
    let rows = count.div_ceil(cols as usize) as u32;
    let (cw, ch) = (400i32, 300i32);
    let (pw, ph) = (cols as i32 * cw, rows as i32 * ch + 40);

    // SAFETY: 単一 WebView 版と同じ Win32 標準契約。class/hwnd は本スレッド内で完結。
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
            w!("plugin-gui-poc multi parent"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            pw,
            ph,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };

    // 各 WebView の done 統計を idx 付きで集約。
    let done_map: Rc<RefCell<HashMap<usize, serde_json::Value>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let loaded = Rc::new(Cell::new(0usize));
    let start = Instant::now();

    let bench_html = |idx: usize| -> String {
        format!(
            r#"<!doctype html><title>pgui-bench {idx}</title>
            <body style="margin:0;background:#181820;overflow:hidden">
            <canvas id="c" width="380" height="280"></canvas>
            <script>
            const cv=document.getElementById('c'),ctx=cv.getContext('2d');
            const FRAMES={frames},IDX={idx};
            let prev,times=[],work=[],n=0;
            function draw(t){{
             ctx.fillStyle='#181820';ctx.fillRect(0,0,cv.width,cv.height);
             for(let i=0;i<24;i++){{
              const a=t*0.001*(1+i*0.07)+i;
              const cx=40+(i%6)*60,cy=50+((i/6)|0)*60;
              ctx.strokeStyle='hsl('+((i*40+t*0.05)%360)+',70%,60%)';ctx.lineWidth=3;
              ctx.beginPath();ctx.arc(cx,cy,18,a,a+Math.PI*1.5);ctx.stroke();
              ctx.fillStyle='#ccc';ctx.fillRect(cx-2,cy-18,4,10*Math.abs(Math.sin(a))+2);
             }}
            }}
            function loop(ts){{
             if(prev!==undefined&&n>=12)times.push(ts-prev);
             prev=ts;
             const w0=performance.now();draw(ts);work.push(performance.now()-w0);
             n++;
             if(n<FRAMES+12){{requestAnimationFrame(loop);}}
             else{{
              const avg=a=>a.length?a.reduce((x,y)=>x+y,0)/a.length:0;
              const dropped=times.filter(t=>t>33.4).length;
              window.ipc.postMessage(JSON.stringify({{type:'done',idx:IDX,
               frames:times.length,dropped,frame_avg_ms:avg(times),
               work_avg_ms:avg(work)}}));
             }}
            }}
            requestAnimationFrame(loop);
            </script></body>"#,
            frames = frames,
            idx = idx
        )
    };

    let mut webviews = Vec::new();
    let mut attach_failed = 0usize;
    for i in 0..count {
        let col = (i % cols as usize) as i32;
        let row = (i / cols as usize) as i32;
        // SAFETY: 子 HWND は親 hwnd 直下に生成。Win32 標準契約。
        let child = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("plugin_gui_poc_parent"),
                w!("child"),
                WS_CHILD | WS_VISIBLE,
                col * cw,
                row * ch + 40,
                cw,
                ch,
                Some(hwnd),
                None,
                Some(GetModuleHandleW(None).unwrap().into()),
                None,
            )
            .unwrap()
        };
        let parent = HostWindow(child);
        let done_map = Rc::clone(&done_map);
        let loaded = Rc::clone(&loaded);
        let r = WebViewBuilder::new()
            .with_html(&bench_html(i))
            .with_ipc_handler(move |req| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(req.body()) {
                    if v.get("type").and_then(|t| t.as_str()) == Some("done") {
                        let idx =
                            v.get("idx")
                                .and_then(|x| x.as_u64())
                                .unwrap_or(usize::MAX as u64) as usize;
                        done_map.borrow_mut().insert(idx, v);
                    }
                }
            })
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, PageLoadEvent::Finished) {
                    loaded.set(loaded.get() + 1);
                }
            })
            .with_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(PhysicalSize::new(cw as u32, ch as u32)),
            })
            .build_as_child(&parent);
        match r {
            Ok(w) => webviews.push(w),
            Err(e) => {
                attach_failed += 1;
                m.emit("multi_attach", "failed", &format!("child {i}: {e}"));
            }
        }
    }
    m.emit(
        "multi_attach",
        if attach_failed == 0 {
            "passed"
        } else {
            "failed"
        },
        &format!("{} of {} webviews attached", webviews.len(), count),
    );

    // メッセージループ: 全 WebView の done 受信 or タイムアウトで終了。
    let mut msg = MSG::default();
    let result = loop {
        let mut quit = false;
        // SAFETY: 標準 Win32 メッセージ処理。
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    quit = true;
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if quit {
            break "quit";
        }
        let done = done_map.borrow().len();
        if done >= count {
            break "done";
        }
        if start.elapsed().as_secs() > duration_secs {
            break "timeout";
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let _ = webviews; // Drop でデタッチされるまで生存させる（本ループ終了時点で破棄）

    let map = done_map.borrow();
    let n = map.len().max(1) as f64;
    let sum = |k: &str| -> f64 {
        map.values()
            .filter_map(|v| v.get(k).and_then(|x| x.as_f64()))
            .sum()
    };
    let min_frames = map
        .values()
        .filter_map(|v| v.get("frames").and_then(|x| x.as_u64()))
        .min()
        .unwrap_or(0);
    m.emit(
        "multi_done",
        if result == "done" { "passed" } else { "failed" },
        &format!("{} of {} webviews done ({result})", map.len(), count),
    );
    m.emit("multi_min_frames", "passed", &min_frames.to_string());
    m.emit(
        "multi_dropped_total",
        "passed",
        &(sum("dropped") as u64).to_string(),
    );
    m.emit(
        "multi_frame_avg_ms",
        "passed",
        &format!("{:.3}", sum("frame_avg_ms") / n),
    );
    m.emit(
        "multi_work_avg_ms",
        "passed",
        &format!("{:.3}", sum("work_avg_ms") / n),
    );
    m.emit(
        "multi_mem_delta_bytes",
        "passed",
        &(working_set_bytes() as i64 - mem_before as i64).to_string(),
    );
    m.emit(
        "multi_elapsed_ms",
        "passed",
        &start.elapsed().as_millis().to_string(),
    );
    if result == "done" && attach_failed == 0 {
        0
    } else {
        1
    }
}

/// Issue #57 Linux 実機相当検証 — wry の Linux 子 attach は
/// `build_as_child`（HWND）ではなく `build_gtk`（gtk::Container）経路。
/// ホスト提供コンテナの模倣として gtk::Fixed を親に使い、Windows 版と
/// 同一の e1_*/e2_* メトリクスを採取する。
#[cfg(target_os = "linux")]
fn run() -> i32 {
    use gtk::prelude::*;
    use wry::WebViewBuilderExtUnix;

    let out = std::env::args()
        .position(|a| a == "--out")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|p| File::create(p).ok());
    let mut m = Metrics { out };

    if gtk::init().is_err() {
        m.emit("e1_attach", "failed", "gtk::init failed (no display)");
        return 1;
    }
    let mem_before = working_set_bytes();
    let start = Instant::now();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("plugin-gui-poc host parent");
    window.set_default_size(800, 600);
    let fixed = gtk::Fixed::new();
    window.add(&fixed);
    window.show_all();

    // WebView をホスト提供コンテナ（gtk::Fixed）へ子として attach。
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
            .build_gtk(&fixed)
    };
    let webview: WebView = match wv_result {
        Ok(w) => {
            m.emit("e1_attach", "passed", "build_gtk on gtk::Fixed host");
            w
        }
        Err(e) => {
            m.emit("e1_attach", "failed", &format!("{e}"));
            return 1;
        }
    };
    window.show_all();

    // glib メインループ — 50ms 周期で load → resize → IPC を検査し、
    // 実験完了か 20s で終了。
    let done_flag = Rc::new(Cell::new(false));
    {
        let m = Rc::new(RefCell::new(m));
        let done_flag = Rc::clone(&done_flag);
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let mut m = m.borrow_mut();
            let mut resize_done = RESIZE_DONE.with(|c| c.get());
            if loaded.get() && !resize_done {
                resize_done = true;
                RESIZE_DONE.with(|c| c.set(true));
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
            if ipc_got.get() && !IPC_CHECKED.with(|c| c.get()) {
                IPC_CHECKED.with(|c| c.set(true));
                m.emit("e1_ipc", "passed", "window.ipc.postMessage ping received");
            }
            if loaded.get() && resize_done && IPC_CHECKED.with(|c| c.get()) {
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
                gtk::main_quit();
                return glib::ControlFlow::Break;
            }
            if start.elapsed().as_secs() > 20 {
                m.emit("e1_ipc", "failed", "timeout waiting for ipc");
                done_flag.set(true);
                gtk::main_quit();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
    gtk::main();
    if done_flag.get() { 1 } else { 0 }
}

#[cfg(target_os = "linux")]
thread_local! {
    static RESIZE_DONE: Cell<bool> = const { Cell::new(false) };
    static IPC_CHECKED: Cell<bool> = const { Cell::new(false) };
}

/// Linux 版複数 WebView 共存実験 — ホスト提供親コンテナ
/// （gtk::Fixed）に `build_gtk` で N 個の WebView を格子状に attach。
/// bounds の (x, y) が Fixed::put の座標になるためセル配置に使う。
#[cfg(target_os = "linux")]
fn run_multi(count: usize, frames: u32, duration_secs: u64, out: Option<File>) -> i32 {
    use gtk::prelude::*;
    use wry::WebViewBuilderExtUnix;

    let mut m = Metrics { out };
    if gtk::init().is_err() {
        m.emit("multi_attach", "failed", "gtk::init failed (no display)");
        return 1;
    }
    let mem_before = working_set_bytes();

    let cols = (count as f64).sqrt().ceil() as u32;
    let rows = count.div_ceil(cols as usize) as u32;
    let (cw, ch) = (400i32, 300i32);
    let (pw, ph) = (cols as i32 * cw, rows as i32 * ch + 40);

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("plugin-gui-poc multi parent");
    window.set_default_size(pw, ph);
    let fixed = gtk::Fixed::new();
    window.add(&fixed);
    window.show_all();

    let done_map: Rc<RefCell<HashMap<usize, serde_json::Value>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let start = Instant::now();

    let bench_html = |idx: usize| -> String {
        format!(
            r#"<!doctype html><title>pgui-bench {idx}</title>
            <body style="margin:0;background:#181820;overflow:hidden">
            <canvas id="c" width="380" height="280"></canvas>
            <script>
            const cv=document.getElementById('c'),ctx=cv.getContext('2d');
            const FRAMES={frames},IDX={idx};
            let prev,times=[],work=[],n=0;
            function draw(t){{
             ctx.fillStyle='#181820';ctx.fillRect(0,0,cv.width,cv.height);
             for(let i=0;i<24;i++){{
              const a=t*0.001*(1+i*0.07)+i;
              const cx=40+(i%6)*60,cy=50+((i/6)|0)*60;
              ctx.strokeStyle='hsl('+((i*40+t*0.05)%360)+',70%,60%)';ctx.lineWidth=3;
              ctx.beginPath();ctx.arc(cx,cy,18,a,a+Math.PI*1.5);ctx.stroke();
              ctx.fillStyle='#ccc';ctx.fillRect(cx-2,cy-18,4,10*Math.abs(Math.sin(a))+2);
             }}
            }}
            function loop(ts){{
             if(prev!==undefined&&n>=12)times.push(ts-prev);
             prev=ts;
             const w0=performance.now();draw(ts);work.push(performance.now()-w0);
             n++;
             if(n<FRAMES+12){{requestAnimationFrame(loop);}}
             else{{
              const avg=a=>a.length?a.reduce((x,y)=>x+y,0)/a.length:0;
              const dropped=times.filter(t=>t>33.4).length;
              window.ipc.postMessage(JSON.stringify({{type:'done',idx:IDX,
               frames:times.length,dropped,frame_avg_ms:avg(times),
               work_avg_ms:avg(work)}}));
             }}
            }}
            requestAnimationFrame(loop);
            </script></body>"#,
            frames = frames,
            idx = idx
        )
    };

    let mut webviews = Vec::new();
    let mut attach_failed = 0usize;
    for i in 0..count {
        let col = (i % cols as usize) as i32;
        let row = (i / cols as usize) as i32;
        let done_map = Rc::clone(&done_map);
        let r = WebViewBuilder::new()
            .with_html(&bench_html(i))
            .with_ipc_handler(move |req| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(req.body()) {
                    if v.get("type").and_then(|t| t.as_str()) == Some("done") {
                        let idx =
                            v.get("idx")
                                .and_then(|x| x.as_u64())
                                .unwrap_or(usize::MAX as u64) as usize;
                        done_map.borrow_mut().insert(idx, v);
                    }
                }
            })
            .with_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(col * cw, row * ch + 40)),
                size: Size::Physical(PhysicalSize::new(cw as u32, ch as u32)),
            })
            .build_gtk(&fixed);
        match r {
            Ok(w) => webviews.push(w),
            Err(e) => {
                attach_failed += 1;
                m.emit("multi_attach", "failed", &format!("child {i}: {e}"));
            }
        }
    }
    window.show_all();
    m.emit(
        "multi_attach",
        if attach_failed == 0 {
            "passed"
        } else {
            "failed"
        },
        &format!("{} of {} webviews attached", webviews.len(), count),
    );

    let result = Rc::new(Cell::new("timeout"));
    {
        let result = Rc::clone(&result);
        let done_map = Rc::clone(&done_map);
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if done_map.borrow().len() >= count {
                result.set("done");
                gtk::main_quit();
                return glib::ControlFlow::Break;
            }
            if start.elapsed().as_secs() > duration_secs {
                gtk::main_quit();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
    gtk::main();
    let _ = webviews; // Drop でデタッチされるまで生存させる
    let result = result.get();

    let map = done_map.borrow();
    let n = map.len().max(1) as f64;
    let sum = |k: &str| -> f64 {
        map.values()
            .filter_map(|v| v.get(k).and_then(|x| x.as_f64()))
            .sum()
    };
    let min_frames = map
        .values()
        .filter_map(|v| v.get("frames").and_then(|x| x.as_u64()))
        .min()
        .unwrap_or(0);
    m.emit(
        "multi_done",
        if result == "done" { "passed" } else { "failed" },
        &format!("{} of {} webviews done ({result})", map.len(), count),
    );
    m.emit("multi_min_frames", "passed", &min_frames.to_string());
    m.emit(
        "multi_dropped_total",
        "passed",
        &(sum("dropped") as u64).to_string(),
    );
    m.emit(
        "multi_frame_avg_ms",
        "passed",
        &format!("{:.3}", sum("frame_avg_ms") / n),
    );
    m.emit(
        "multi_work_avg_ms",
        "passed",
        &format!("{:.3}", sum("work_avg_ms") / n),
    );
    m.emit(
        "multi_mem_delta_bytes",
        "passed",
        &(working_set_bytes() as i64 - mem_before as i64).to_string(),
    );
    m.emit(
        "multi_elapsed_ms",
        "passed",
        &start.elapsed().as_millis().to_string(),
    );
    if result == "done" && attach_failed == 0 {
        0
    } else {
        1
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn run_multi(_count: usize, _frames: u32, _duration_secs: u64, _out: Option<File>) -> i32 {
    eprintln!("plugin-gui-poc: windows/linux only (this host is skipped)");
    2
}

#[cfg(not(any(windows, target_os = "linux")))]
fn run() -> i32 {
    eprintln!("plugin-gui-poc: windows/linux only (this host is skipped)");
    2
}

fn arg_value(flag: &str) -> Option<String> {
    std::env::args()
        .position(|a| a == flag)
        .and_then(|i| std::env::args().nth(i + 1))
}

fn main() {
    let out = arg_value("--out").and_then(|p| File::create(p).ok());
    if let Some(count) = arg_value("--count") {
        let count: usize = count.parse().unwrap_or(0);
        let frames: u32 = arg_value("--frames")
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        let duration: u64 = arg_value("--duration")
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        if count == 0 || count > 16 {
            eprintln!("plugin-gui-poc: --count must be 1..=16");
            std::process::exit(2);
        }
        std::process::exit(run_multi(count, frames, duration, out));
    }
    // 既存の単一 WebView 実験（`--out` は既に消費済みなので run() 側で再解決しない）。
    let _ = out;
    std::process::exit(run());
}
