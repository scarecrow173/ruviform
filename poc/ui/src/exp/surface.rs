//! `--experiment surface`（AC3: ネイティブ GPU サーフェス合成）。
//!
//! **adapter リハーサルの例外モジュール**: 他の `exp/*` が host.rs の内部 I/F
//! （`Host`/`HostEvent`）だけを使うのに対し、本モジュールは AC3 の検証対象自体が
//! HWND レベルの合成（wgpu Surface・sibling 子 HWND・z-order・リージョン）なため、
//! tao/wry/wgpu/windows-rs を直接保持する（design「wgpu・windows-rs はこの
//! モジュールに封じる — 将来 ruviform-gpu adapter のリハーサル」）。
//! 「汎用 host I/F ではネイティブ合成実験を表現できない」こと自体が AC3 の
//! 評価軸の発見として RESULTS.md に記録する。
//!
//! モード（design 正本）:
//! - `adjacent-parent`: tao 親 HWND 自体を wgpu Surface 化し、WebView を
//!   `build_as_child` で左領域に配置。子 HWND は親描画の上に来るため
//!   追加 HWND なしで隣接同居を構成する（unsafe 不要の主経路）。
//! - `adjacent-child`: WebView を `build` + `set_bounds` で左領域に配置し、
//!   windows-rs `WS_CHILD` の sibling 子 HWND へ wgpu を張る。兄弟 z-order・
//!   WebView 領域への一時重なりを検査する（unsafe は windows-rs /
//!   `create_surface_unsafe` 呼出箇所の SAFETY コメント付き局所のみ）。
//!
//! 検査マトリクス（各 pass/fail + 観察を記録）:
//!   surface_init / webview_child / hit_test_webview / hit_test_native /
//!   focus_transition / z_order_overlap（child のみ）/ resize_follow /
//!   frame_continuity / region_clip（SetWindowRgn 試行）。
//! ヒットテストは `WindowFromPoint` + `IsChild` の静的判定を使う
//! （合成クリック入力は PoC 範囲外。判定方法は観察欄に記録）。

use serde_json::json;

use crate::args::RunArgs;
use crate::check::Environment;
use crate::metrics::Status;

use super::{Experiment, Outcome, Skip, check_gui_prereqs};

/// AC3 実験。状態は `run` 内に閉じる。
pub struct SurfaceExperiment;

/// `--mode` が取りうる値。
const MODES: [&str; 2] = ["adjacent-parent", "adjacent-child"];

impl SurfaceExperiment {
    /// 実験インスタンス生成（レジストリから呼ばれる）。
    pub fn new() -> Self {
        Self
    }
}

impl Experiment for SurfaceExperiment {
    fn name(&self) -> &'static str {
        "surface"
    }

    fn setup(&mut self, env: &Environment) -> Result<(), Skip> {
        check_gui_prereqs(env)?;
        // AC3 は wgpu ネイティブ描画が前提。adapter 非検出は前提不足。
        if env.wgpu_adapter.is_none() {
            return Err(Skip("no wgpu adapter detected".to_string()));
        }
        Ok(())
    }

    fn run(&mut self, _env: &Environment, args: &RunArgs) -> Outcome {
        let mode = match args.mode.as_deref() {
            Some(m) if MODES.contains(&m) => m.to_string(),
            Some(other) => {
                return Outcome {
                    metrics: json!({"mode": other}),
                    status: Status::Failed(format!(
                        "invalid --mode '{other}' (expected adjacent-parent|adjacent-child)"
                    )),
                };
            }
            None => {
                return Outcome {
                    metrics: json!({}),
                    status: Status::Failed(
                        "missing --mode (expected adjacent-parent|adjacent-child)".to_string(),
                    ),
                };
            }
        };
        win::run(&mode, args)
    }

    fn teardown(&mut self) {}
}

/// Windows 専用実装（wgpu・windows-rs・wry 子ビューを内側に封じる）。
/// 非 Windows では setup 済みでも本実験は未対応として skipped を返す。
#[cfg(windows)]
mod win {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::num::NonZeroIsize;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};
    use raw_window_handle::{
        HasWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
    };
    use serde_json::{Value, json};
    use tao::event::{Event, StartCause, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::platform::run_return::EventLoopExtRunReturn;
    use tao::window::{Window, WindowBuilder};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, CreateRectRgn, DeleteObject, HGDIOBJ, HRGN, SetWindowRgn,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, EnumChildWindows, GetClassNameW, GetParent, GetWindowRect,
        IsChild, RegisterClassW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SetWindowPos, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS,
        WS_VISIBLE, WindowFromPoint,
    };
    use windows::core::BOOL;
    use windows::core::w;
    use wry::WebViewExtWindows;
    use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

    use crate::args::RunArgs;
    use crate::assets;
    use crate::metrics::Status;

    use super::Outcome;

    /// 実験駆動の周期起床間隔（wgpu フレーム駆動 ≒ 60fps 目安。
    /// Windows タイマ分解能 ~15.6ms で実配信は丸められる — 実測する）。
    const TICK: Duration = Duration::from_millis(16);
    /// 既定の総タイムアウト（段階列 + 余裕）。
    const DEFAULT_TIMEOUT_SECS: u64 = 30;
    /// WebView の占有幅比率（左領域）。
    const WEBVIEW_FRACTION: f64 = 0.5;

    /// キューに載せる正規化イベント（wry callback → ループ内処理の橋渡し）。
    enum Ev {
        Ipc(String),
        PageLoad(String),
    }

    /// wgpu レンダラ（surface + device + queue + 最小三角パイプライン）。
    struct Gpu {
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        pipeline: wgpu::RenderPipeline,
        /// adjacent-parent では親 surface がクライアント全域を覆うため、
        /// 三角を右半分 NDC に寄せる。adjacent-child では surface が右領域の
        /// 子 HWND に限定されるため全面を使う。
        right_half_only: bool,
    }

    impl Gpu {
        /// `hwnd`（親または sibling 子）に Surface を構築して描画準備を整える。
        #[expect(
            unsafe_code,
            reason = "raw HWND → wgpu unsafe surface。SAFETY 各所を参照"
        )]
        fn new(hwnd: HWND, width: u32, height: u32, right_half_only: bool) -> Result<Self, String> {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let wh = Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).ok_or("null hwnd")?);
            // SAFETY: `hwnd` はこの実験が生成した有効なウィンドウ/子 HWND で、
            // surface は hwnd 生存期間内のみ使い hwnd 破棄前に drop する
            // （`run_surface` のスコープ内で win/webview より先に破棄される）。
            let surface = unsafe {
                instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle: Some(RawDisplayHandle::Windows(
                            WindowsDisplayHandle::new(),
                        )),
                        raw_window_handle: RawWindowHandle::Win32(wh),
                    })
                    .map_err(|e| format!("create_surface_unsafe failed: {e}"))?
            };
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                }))
                .map_err(|e| format!("request_adapter failed: {e}"))?;
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                    .map_err(|e| format!("request_device failed: {e}"))?;
            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: width.max(1),
                height: height.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            // 三角 + 位相回転クリア色（フレーム継続の可視確認 + present 実績）。
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("poc-triangle"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    var pos = array<vec2f, 3>(vec2f(0.1, -0.8), vec2f(0.9, -0.8), vec2f(0.5, 0.8));
    return vec4f(pos[i], 0.0, 1.0);
}
@fragment
fn fs() -> @location(0) vec4f {
    return vec4f(0.15, 0.7, 0.9, 1.0);
}
"#
                    .into(),
                ),
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("poc-triangle"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            Ok(Self {
                surface,
                device,
                queue,
                config,
                pipeline,
                right_half_only,
            })
        }

        /// 1 フレーム描画（アニメーションクリア色 + 三角）。
        /// `Ok(true)`=present 済み、`Ok(false)`=スキップ（occluded/timeout）、
        /// `Err`=取得/描画の失敗。
        fn frame(&self, n: u64) -> Result<bool, String> {
            use wgpu::CurrentSurfaceTexture as Cst;
            let out = match self.surface.get_current_texture() {
                Cst::Success(t) | Cst::Suboptimal(t) => t,
                Cst::Outdated | Cst::Lost => {
                    self.surface.configure(&self.device, &self.config);
                    match self.surface.get_current_texture() {
                        Cst::Success(t) | Cst::Suboptimal(t) => t,
                        other => {
                            return Err(format!("reconfigure+get_current_texture -> {other:?}"));
                        }
                    }
                }
                Cst::Timeout | Cst::Occluded => return Ok(false),
                Cst::Validation => return Err("surface validation error".to_string()),
            };
            let view = out.texture.create_view(&Default::default());
            let mut enc = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("poc") });
            {
                let hue = (n % 240) as f64 / 240.0;
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("poc"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: hue * 0.4,
                                g: 0.08,
                                b: 0.18 + hue * 0.2,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                if self.right_half_only {
                    // 親 surface 全域の右半分だけに描画（左は WebView 子が覆う）。
                    let w = self.config.width / 2;
                    pass.set_viewport(
                        (self.config.width - w) as f32,
                        0.0,
                        w as f32,
                        self.config.height as f32,
                        0.0,
                        1.0,
                    );
                }
                pass.set_pipeline(&self.pipeline);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit([enc.finish()]);
            // wgpu 30: present は SurfaceTexture のメソッドではなく Queue::present。
            self.queue.present(out);
            Ok(true)
        }

        /// リサイズ追従（`surface.configure`）。
        fn resize(&mut self, w: u32, h: u32) {
            if w > 0 && h > 0 {
                self.config.width = w;
                self.config.height = h;
                self.surface.configure(&self.device, &self.config);
            }
        }
    }

    /// sibling 子 HWND（adjacent-child 用）の RAII ラッパ。
    struct ChildHwnd(HWND);

    /// 子 HWND の最小 wndproc（windows 0.62 の `DefWindowProcW` は Rust ABI の
    /// ため system ABI の薄いラッパを噛ませる）。
    #[expect(unsafe_code, reason = "Win32 wndproc は unsafe extern fn が必須")]
    unsafe extern "system" fn child_wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // SAFETY: 全メッセージを既定処理へ委譲する。
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    impl ChildHwnd {
        /// 親クライアント座標 (x,y,w,h) に WS_CHILD ウィンドウを生成する。
        #[expect(
            unsafe_code,
            reason = "windows-rs Win32 呼出。SAFETY: 引数は検証済み値、生成物は Drop で破棄"
        )]
        fn create(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<Self, String> {
            // SAFETY: GetModuleHandleW(None) は自プロセスの HINSTANCE を返す。
            let hinst = unsafe { GetModuleHandleW(None) }
                .map_err(|e| format!("GetModuleHandleW failed: {e}"))?;
            let class = w!("ui_poc_gpu_child");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(child_wndproc),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            // SAFETY: 同一クラス名の再登録は ERROR_CLASS_ALREADY_EXISTS で
            // 失敗するだけなので戻り値は無視してよい。
            let _ = unsafe { RegisterClassW(&wc) };
            // SAFETY: 全引数は本実験内で構成した有効値。失敗時は Err。
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class,
                    w!("gpu"),
                    WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0),
                    x,
                    y,
                    w.max(1),
                    h.max(1),
                    Some(parent),
                    None,
                    Some(hinst.into()),
                    None,
                )
            }
            .map_err(|e| format!("CreateWindowExW failed: {e}"))?;
            Ok(Self(hwnd))
        }

        fn hwnd(&self) -> HWND {
            self.0
        }

        /// sibling を (x,y,w,h) へ移動する（z-order は変えない）。
        #[expect(
            unsafe_code,
            reason = "SetWindowPos は Win32 FFI。SAFETY: 有効 HWND のみ"
        )]
        fn move_to(&self, x: i32, y: i32, w: i32, h: i32) {
            // SAFETY: self.0 は生存中の子 HWND。
            unsafe {
                let _ = SetWindowPos(
                    self.0,
                    None,
                    x,
                    y,
                    w.max(1),
                    h.max(1),
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
        }

        /// sibling を兄弟 z-order の最上位へ上げる（hwndInsertAfter=NULL は
        /// HWND_TOP）。オーバーラップ試験用。
        #[expect(
            unsafe_code,
            reason = "SetWindowPos は Win32 FFI。SAFETY: 有効 HWND のみ"
        )]
        fn bring_to_top(&self) {
            // SAFETY: self.0 は生存中の子 HWND。None(HWND_TOP)=最上位指定。
            unsafe {
                let _ = SetWindowPos(
                    self.0,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                );
            }
        }
    }

    impl Drop for ChildHwnd {
        #[expect(
            unsafe_code,
            reason = "DestroyWindow は Win32 FFI。SAFETY: 所有 HWND のみ"
        )]
        fn drop(&mut self) {
            // SAFETY: 本実験が生成した子 HWND のみ破棄する。
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0);
            }
        }
    }

    /// クライアント座標 (x,y) にある HWND を返す（ヒットテストの静的判定）。
    #[expect(
        unsafe_code,
        reason = "ClientToScreen/WindowFromPoint は Win32 FFI。SAFETY: 有効 HWND"
    )]
    fn hwnd_at(parent: HWND, client_x: i32, client_y: i32) -> Option<HWND> {
        let mut pt = POINT {
            x: client_x,
            y: client_y,
        };
        // SAFETY: parent は生存中のウィンドウ。
        let _ = unsafe { ClientToScreen(parent, &mut pt) };
        // SAFETY: WindowFromPoint は純粋な照会。
        let h = unsafe { WindowFromPoint(pt) };
        if h.is_invalid() { None } else { Some(h) }
    }

    /// `found` が `expected` と同一またはその子孫なら true（WebView2 は
    /// 内部に deeper HWND を持つため祖先方向も含めて判定する）。
    #[expect(unsafe_code, reason = "IsChild は Win32 FFI。SAFETY: 照会のみ")]
    fn hwnd_matches(found: HWND, expected: HWND) -> bool {
        if found == expected {
            return true;
        }
        // SAFETY: 照会のみ。
        unsafe { IsChild(expected, found).as_bool() || IsChild(found, expected).as_bool() }
    }

    /// HWND のウィンドウクラス名（診断用）。
    #[expect(unsafe_code, reason = "GetClassNameW は Win32 FFI。SAFETY: 照会のみ")]
    fn class_of(hwnd: HWND) -> String {
        let mut buf = [0u16; 96];
        // SAFETY: buf は 96 要素の有効なバッファ、hwnd は生存中の HWND か null。
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    /// 親の子 HWND ツリーを列挙（クラス名・矩形・親 HWND）。判定不能時の
    /// 一次資料として metrics.children に記録する。
    #[expect(
        unsafe_code,
        reason = "EnumChildWindows/GetWindowRect/GetParent は Win32 FFI。SAFETY: 照会のみ"
    )]
    fn dump_children(parent: HWND) -> Vec<Value> {
        struct Ctx {
            out: Vec<Value>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            // SAFETY: lp は呼出元の `&mut Ctx` を指す。EnumChildWindows の同期
            // コール中のみ有効で、脱出前に元の Vec へ戻す。
            let ctx = unsafe { &mut *(lp.0 as *mut Ctx) };
            let mut cls = [0u16; 96];
            // SAFETY: hwnd は列挙中の有効な HWND。
            let n = unsafe { GetClassNameW(hwnd, &mut cls) };
            let mut r = RECT::default();
            // SAFETY: 同上。
            let _ = unsafe { GetWindowRect(hwnd, &mut r) };
            // SAFETY: 同上。
            let p = unsafe { GetParent(hwnd) };
            ctx.out.push(json!({
                "hwnd": format!("{hwnd:?}"),
                "class": String::from_utf16_lossy(&cls[..n.max(0) as usize]),
                "rect": [r.left, r.top, r.right, r.bottom],
                "parent": format!("{p:?}"),
            }));
            true.into()
        }
        let mut ctx = Ctx { out: Vec::new() };
        // SAFETY: cb は列挙中のみ ctx を指し、呼出は同期で完結する。
        unsafe {
            let _ = EnumChildWindows(
                Some(parent),
                Some(cb),
                LPARAM(&mut ctx as *mut Ctx as isize),
            );
        }
        ctx.out
    }

    /// クライアント座標 (cx,cy) を覆う自ツリー内の子 HWND を z-order 順に返す
    /// （先頭 = 最上位）。WindowFromPoint と違い他プロセスの重なりを除外できる
    /// ため、実デスクトップでの静的ヒットテストの一次判定に使う。
    /// 戻り値は (hwnd, class) の列。空 = その点では親自身が前面。
    #[expect(
        unsafe_code,
        reason = "EnumChildWindows/GetWindowRect は Win32 FFI。SAFETY: 照会のみ"
    )]
    fn covering_children(parent: HWND, cx: i32, cy: i32) -> Vec<(HWND, String)> {
        struct Ctx {
            pt: POINT,
            hits: Vec<(HWND, String)>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            // SAFETY: lp は呼出元の &mut Ctx。列挙は同期で完結する。
            let ctx = unsafe { &mut *(lp.0 as *mut Ctx) };
            let mut r = RECT::default();
            // SAFETY: hwnd は列挙中の有効な HWND。
            if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok()
                && ctx.pt.x >= r.left
                && ctx.pt.x < r.right
                && ctx.pt.y >= r.top
                && ctx.pt.y < r.bottom
            {
                ctx.hits.push((hwnd, class_of(hwnd)));
            }
            true.into()
        }
        let mut pt = POINT { x: cx, y: cy };
        // SAFETY: parent は生存中のウィンドウ。
        let _ = unsafe { ClientToScreen(parent, &mut pt) };
        let mut ctx = Ctx {
            pt,
            hits: Vec::new(),
        };
        // SAFETY: 同上（同期列挙）。
        unsafe {
            let _ = EnumChildWindows(
                Some(parent),
                Some(cb),
                LPARAM(&mut ctx as *mut Ctx as isize),
            );
        }
        ctx.hits
    }

    /// WebView 子 HWND に `SetWindowRgn` で矩形容器を設定する（DOM overlay
    /// 回避策の試行）。`None` でリージョン解除（全面表示に戻す）。
    #[expect(
        unsafe_code,
        reason = "CreateRectRgn/SetWindowRgn/DeleteObject は Win32 FFI。SAFETY 各所を参照"
    )]
    fn clip_hwnd(hwnd: HWND, rect: Option<(i32, i32, i32, i32)>) -> Result<(), String> {
        match rect {
            Some((l, t, r, b)) => {
                // SAFETY: 有効な矩形値。失敗時は無効ハンドルを返す。
                let region: HRGN = unsafe { CreateRectRgn(l, t, r, b) };
                if region.is_invalid() {
                    return Err("CreateRectRgn returned invalid region".to_string());
                }
                // SAFETY: SetWindowRgn 成功時はシステムがリージョン所有権を持つ
                // （以後 DeleteObject 不可）。失敗時のみ自分で破棄する。
                let ok = unsafe { SetWindowRgn(hwnd, Some(region), true) };
                if ok == 0 {
                    // SAFETY: 所有権が移らなかった失敗時のみ自前で破棄。
                    unsafe {
                        let _ = DeleteObject(HGDIOBJ(region.0));
                    }
                    return Err(format!("SetWindowRgn failed: {ok}"));
                }
                Ok(())
            }
            None => {
                // SAFETY: None でリージョン解除（所有権移転なし）。
                unsafe { SetWindowRgn(hwnd, None, true) };
                Ok(())
            }
        }
    }

    /// 検査マトリクスの 1 項目。
    #[derive(Clone)]
    struct Check {
        result: &'static str, // "pass" | "fail" | "obs"
        note: String,
    }

    impl Check {
        fn pass(note: impl Into<String>) -> Self {
            Self {
                result: "pass",
                note: note.into(),
            }
        }
        fn fail(note: impl Into<String>) -> Self {
            Self {
                result: "fail",
                note: note.into(),
            }
        }
        fn obs(note: impl Into<String>) -> Self {
            Self {
                result: "obs",
                note: note.into(),
            }
        }
        fn to_json(&self) -> Value {
            json!({"result": self.result, "note": self.note})
        }
    }

    /// 段階駆動の検査シーケンス状態。
    struct S {
        stage: u32,
        /// 現在 stage の経過 tick（デッドライン判定用）。
        stage_ticks: u32,
        frames: u64,
        /// get_current_texture が timeout/occluded でスキップした回数。
        frames_skipped: u64,
        frame_errors: Vec<String>,
        hello_seen: bool,
        resize_events: Vec<(u32, u32)>,
        /// JS `resize_observed` の末尾（viewport 追従の真値）。
        js_resizes: Vec<(u32, u32)>,
        focus_events: Vec<bool>,
        /// 検査マトリクス（挿入順 = 実施順）。
        matrix: Vec<(String, Check)>,
        webview_hwnd: Option<HWND>,
        native_hwnd: Option<HWND>,
        /// stage 1 で列挙した子 HWND ツリー（クラス名付き診断）。
        children: Vec<Value>,
        /// resize 試験で要求したサイズ。
        resize_target: Option<(u32, u32)>,
        /// focus 試験で blur→focus のペアを観測できたか。
        focus_pair: bool,
        /// SetWindowRgn の試行結果メモ。
        clip_note: Option<String>,
        errors: Vec<String>,
        exit_reason: Option<&'static str>,
        timeout_hit: bool,
        done: bool,
    }

    impl S {
        fn record(&mut self, name: &str, c: Check) {
            self.matrix.push((name.to_string(), c));
        }
        fn has(&self, name: &str) -> bool {
            self.matrix.iter().any(|(n, _)| n == name)
        }
    }

    fn left_rect(w: u32, h: u32) -> Rect {
        Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(((w as f64) * WEBVIEW_FRACTION) as u32, h)),
        }
    }

    fn right_rect_px(w: u32, h: u32) -> (i32, i32, i32, i32) {
        let lw = ((w as f64) * WEBVIEW_FRACTION) as i32;
        (lw, 0, (w as i32 - lw).max(1), h as i32)
    }

    /// AC3 本体。`mode` は検証済み値。
    /// unsafe は Gpu::new / ChildHwnd / hwnd 照会・操作の各 helper に局所化済み。
    pub fn run(mode: &str, args: &RunArgs) -> Outcome {
        let timeout = Duration::from_secs(args.duration_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let adjacent_child = mode == "adjacent-child";

        let mut event_loop = EventLoopBuilder::<()>::with_user_event().build();
        let window = match WindowBuilder::new()
            .with_title(format!("ui-poc surface (AC3 {mode})"))
            .with_inner_size(LogicalSize::new(960.0f64, 640.0f64))
            .with_resizable(true)
            .build(&event_loop)
        {
            Ok(w) => w,
            Err(e) => {
                return Outcome {
                    metrics: json!({"mode": mode}),
                    status: Status::Failed(format!("window build failed: {e}")),
                };
            }
        };
        // tao Window → 親 HWND。
        let parent_hwnd = match window.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => HWND(h.hwnd.get() as *mut _),
            _ => {
                return Outcome {
                    metrics: json!({"mode": mode}),
                    status: Status::Failed("window_handle is not Win32".to_string()),
                };
            }
        };
        let initial = window.inner_size();
        let (rx, ry, rw, rh) = right_rect_px(initial.width, initial.height);

        // WebView 子ビュー（左領域）。adjacent-parent は build_as_child、
        // adjacent-child は build + set_bounds（host 実験実績のある経路）。
        let queue: Rc<RefCell<VecDeque<Ev>>> = Rc::new(RefCell::new(VecDeque::new()));
        let proxy = event_loop.create_proxy();
        let data_dir = std::env::temp_dir().join(format!("ui-poc-webview2-{}", std::process::id()));
        let mut web_context = WebContext::new(Some(data_dir));
        let webview = {
            let q_ipc = Rc::clone(&queue);
            let q_load = Rc::clone(&queue);
            let p_ipc = proxy.clone();
            let p_load = proxy.clone();
            let builder = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_url("poc://app/index.html")
                .with_custom_protocol("poc".to_string(), |_id, request| {
                    assets::handle_poc_protocol(&request)
                })
                .with_ipc_handler(move |request| {
                    q_ipc
                        .borrow_mut()
                        .push_back(Ev::Ipc(request.body().clone()));
                    let _ = p_ipc.send_event(());
                })
                .with_on_page_load_handler(move |event, url| {
                    if matches!(event, PageLoadEvent::Finished) {
                        q_load.borrow_mut().push_back(Ev::PageLoad(url));
                        let _ = p_load.send_event(());
                    }
                })
                .with_bounds(left_rect(initial.width, initial.height));
            if adjacent_child {
                // `build`（wry 親 subclass が WM_SIZE → SetBounds(全面) を自動化する
                // ため、resize のたびに実験側が左領域へ再 set_bounds する）。
                match builder.build(&window) {
                    Ok(v) => {
                        let _ = v.set_bounds(left_rect(initial.width, initial.height));
                        v
                    }
                    Err(e) => {
                        return Outcome {
                            metrics: json!({"mode": mode}),
                            status: Status::Failed(format!("webview build failed: {e}")),
                        };
                    }
                }
            } else {
                match builder.build_as_child(&window) {
                    Ok(v) => v,
                    Err(e) => {
                        return Outcome {
                            metrics: json!({"mode": mode}),
                            status: Status::Failed(format!("webview build_as_child failed: {e}")),
                        };
                    }
                }
            }
        };

        // wgpu サーフェス: adjacent-parent は親 HWND、adjacent-child は sibling 子。
        let sibling = if adjacent_child {
            match ChildHwnd::create(parent_hwnd, rx, ry, rw, rh) {
                Ok(c) => Some(c),
                Err(e) => {
                    return Outcome {
                        metrics: json!({"mode": mode}),
                        status: Status::Failed(format!("sibling child HWND failed: {e}")),
                    };
                }
            }
        } else {
            None
        };
        let gpu_target = sibling.as_ref().map(|c| c.hwnd()).unwrap_or(parent_hwnd);
        let (gw, gh) = if adjacent_child {
            (rw as u32, rh as u32)
        } else {
            (initial.width, initial.height)
        };
        let gpu = match Gpu::new(gpu_target, gw, gh, !adjacent_child) {
            Ok(g) => g,
            Err(e) => {
                return Outcome {
                    metrics: json!({"mode": mode}),
                    status: Status::Failed(format!("gpu init failed: {e}")),
                };
            }
        };

        let st = Rc::new(RefCell::new(S {
            stage: 0,
            stage_ticks: 0,
            frames: 0,
            frames_skipped: 0,
            frame_errors: Vec::new(),
            hello_seen: false,
            resize_events: Vec::new(),
            js_resizes: Vec::new(),
            focus_events: Vec::new(),
            matrix: Vec::new(),
            webview_hwnd: Some(webview.hwnd()),
            native_hwnd: Some(gpu_target),
            children: Vec::new(),
            resize_target: None,
            focus_pair: false,
            clip_note: None,
            errors: Vec::new(),
            exit_reason: None,
            timeout_hit: false,
            done: false,
        }));
        st.borrow_mut()
            .record("surface_init", Check::pass("wgpu surface+device+pipeline"));

        let gpu = RefCell::new(gpu);
        let sibling = RefCell::new(sibling);
        let deadline = Instant::now() + timeout;
        let timeout_sent = Cell::new(false);
        let want_exit = Cell::new(false);
        let webview = RefCell::new(webview);
        let window = RefCell::new(window);
        let st2 = Rc::clone(&st);

        event_loop.run_return(move |event, _target, control_flow| {
            let st = &st2;
            // IPC / page-load キューを先に消化。
            loop {
                let next = queue.borrow_mut().pop_front();
                match next {
                    Some(Ev::Ipc(body)) => handle_ipc(&body, &mut st.borrow_mut()),
                    Some(Ev::PageLoad(url)) => {
                        st.borrow_mut().errors.push(format!("page_loaded:{url}"));
                    }
                    None => break,
                }
            }
            match event {
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    let now = Instant::now();
                    if now >= deadline {
                        if !timeout_sent.replace(true) {
                            let mut s = st.borrow_mut();
                            s.timeout_hit = true;
                            s.exit_reason = Some("timeout");
                            s.done = true;
                        }
                        want_exit.set(true);
                    } else {
                        on_tick(
                            &window.borrow(),
                            &webview.borrow(),
                            &gpu,
                            &sibling,
                            &mut st.borrow_mut(),
                            adjacent_child,
                            parent_hwnd,
                        );
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let (w, h) = (size.width, size.height);
                    st.borrow_mut().resize_events.push((w, h));
                    relayout(&webview.borrow(), &gpu, &sibling, adjacent_child, w, h);
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(_),
                    ..
                } => {}
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    st.borrow_mut().exit_reason = Some("close_requested");
                    want_exit.set(true);
                }
                _ => {}
            }
            loop {
                let next = queue.borrow_mut().pop_front();
                match next {
                    Some(Ev::Ipc(body)) => handle_ipc(&body, &mut st.borrow_mut()),
                    Some(Ev::PageLoad(url)) => {
                        st.borrow_mut().errors.push(format!("page_loaded:{url}"));
                    }
                    None => break,
                }
            }
            if st.borrow().done {
                want_exit.set(true);
            }
            if want_exit.get() {
                *control_flow = ControlFlow::Exit;
            } else {
                let now = Instant::now();
                let next_wake = now + TICK;
                *control_flow = ControlFlow::WaitUntil(if next_wake < deadline {
                    next_wake
                } else {
                    deadline
                });
            }
        });

        let s = st.borrow();
        finish_metrics(&s, mode)
    }

    /// リサイズ時の再レイアウト: webview を左領域へ、gpu/sibling を右領域へ。
    fn relayout(
        webview: &WebView,
        gpu: &RefCell<Gpu>,
        sibling: &RefCell<Option<ChildHwnd>>,
        adjacent_child: bool,
        w: u32,
        h: u32,
    ) {
        let _ = webview.set_bounds(left_rect(w, h));
        if adjacent_child {
            let (x, y, cw, ch) = right_rect_px(w, h);
            if let Some(c) = sibling.borrow().as_ref() {
                c.move_to(x, y, cw, ch);
            }
            gpu.borrow_mut().resize(cw as u32, ch as u32);
        } else {
            gpu.borrow_mut().resize(w, h);
        }
    }

    /// IPC（JS 観測イベント）の記録。
    fn handle_ipc(body: &str, s: &mut S) {
        let msg: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                s.errors.push(format!("ipc parse error: {e}"));
                return;
            }
        };
        match msg.get("type").and_then(Value::as_str) {
            Some("hello") => s.hello_seen = true,
            Some("focus") => {
                if let Some(f) = msg.get("focused").and_then(Value::as_bool) {
                    s.focus_events.push(f);
                    // blur(false) → focus(true) の順序ペアを探す。
                    let n = s.focus_events.len();
                    if n >= 2 && !s.focus_events[n - 2] && f {
                        s.focus_pair = true;
                    }
                }
            }
            Some("resize_observed") => {
                let w = msg.get("w").and_then(Value::as_u64).unwrap_or(0) as u32;
                let h = msg.get("h").and_then(Value::as_u64).unwrap_or(0) as u32;
                if s.js_resizes.len() >= 16 {
                    s.js_resizes.remove(0);
                }
                s.js_resizes.push((w, h));
            }
            _ => {}
        }
    }

    /// tick ごとの段階駆動: フレーム描画 + 検査ステップを進める。
    /// unsafe は hwnd_at / hwnd_matches / clip_hwnd / ChildHwnd::move_to に局所化済み。
    fn on_tick(
        window: &Window,
        webview: &WebView,
        gpu: &RefCell<Gpu>,
        sibling: &RefCell<Option<ChildHwnd>>,
        s: &mut S,
        adjacent_child: bool,
        parent_hwnd: HWND,
    ) {
        // フレーム描画（継続性の証跡）。timeout/occluded はスキップ計数。
        match gpu.borrow().frame(s.frames) {
            Ok(true) => s.frames += 1,
            Ok(false) => s.frames_skipped += 1,
            Err(e) => {
                if s.frame_errors.len() < 4 {
                    s.frame_errors.push(e);
                }
            }
        }
        s.stage_ticks += 1;

        let size = window.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let lw = ((w as f64) * WEBVIEW_FRACTION) as i32;
        let wv = s.webview_hwnd.unwrap_or_default();
        let nat = s.native_hwnd.unwrap_or_default();

        match s.stage {
            // stage 0: hello 待ち（最大 100 tick ≒1.6s）。来なければ fail 記録で先へ。
            0 => {
                if s.hello_seen {
                    s.stage = 1;
                    s.stage_ticks = 0;
                } else if s.stage_ticks > 100 {
                    s.record("webview_child", Check::fail("page hello not observed"));
                    s.stage = 1;
                    s.stage_ticks = 0;
                }
            }
            // stage 1: 静定 + 初期検査（30 フレーム経過後）。
            1 => {
                if s.frames < 30 && s.stage_ticks < 200 {
                    return;
                }
                if !s.has("webview_child") {
                    s.record("webview_child", Check::pass("hello observed"));
                }
                // 子 HWND ツリー（クラス名付き）を一次資料として記録。
                s.children = dump_children(parent_hwnd);
                // hit test: 左中央は WebView 系、右中央は native 系であること。
                // 一次判定は「自ツリー内で点を覆う最上位子 HWND」（foreign
                // top-level ウィンドウの混入を排除）。WindowFromPoint の生値は
                // 他アプリ重なりの観察用に併記する。
                let cov = covering_children(parent_hwnd, lw / 2, h / 2);
                let wfp = hwnd_at(parent_hwnd, lw / 2, h / 2);
                let webview_hit = cov
                    .iter()
                    .any(|(hc, c)| hwnd_matches(*hc, wv) || c.starts_with("Chrome_WidgetWin"));
                s.record(
                    "hit_test_webview",
                    if webview_hit {
                        Check::pass(format!(
                            "covering children at left-center: {cov:?}; WindowFromPoint -> {wfp:?}"
                        ))
                    } else {
                        Check::fail(format!(
                            "no webview child covers left-center (cov={cov:?}, wfp={wfp:?})"
                        ))
                    },
                );
                let cov = covering_children(parent_hwnd, lw + (w - lw) / 2, h / 2);
                let wfp = hwnd_at(parent_hwnd, lw + (w - lw) / 2, h / 2);
                // native 到達 = 覆う最上位子が native 対象（adjacent-parent は
                // 子なし=親自身、adjacent-child は sibling）または非 WebView2。
                let top_is_webview = cov
                    .first()
                    .map(|(hc, c)| hwnd_matches(*hc, wv) || c.starts_with("Chrome_WidgetWin"))
                    .unwrap_or(false);
                let native_hit = match cov.first() {
                    None => !adjacent_child, // 親自身が前面（child では sibling が必須）
                    Some((hc, _)) => hwnd_matches(*hc, nat) && !top_is_webview,
                };
                s.record(
                    "hit_test_native",
                    if native_hit {
                        Check::pass(format!(
                            "right-center top covering: {:?}; WindowFromPoint -> {wfp:?}",
                            cov.first()
                        ))
                    } else {
                        Check::fail(format!(
                            "right-center covered by non-native {cov:?} (wfp={wfp:?})"
                        ))
                    },
                );
                // focus: focus_parent → focus の DOM blur/focus ペア観測。
                let _ = webview.focus_parent();
                s.stage = 2;
                s.stage_ticks = 0;
            }
            // stage 2: focus 往復の後半（focus() 投入 → DOM focus 期待）。
            2 => {
                if s.stage_ticks == 2 {
                    let _ = webview.focus();
                }
                if s.stage_ticks > 12 {
                    s.record(
                        "focus_transition",
                        if s.focus_pair {
                            Check::pass(format!(
                                "DOM blur->focus pair observed {:?}",
                                s.focus_events
                            ))
                        } else {
                            Check::fail(format!(
                                "no blur->focus pair (events {:?})",
                                s.focus_events
                            ))
                        },
                    );
                    // resize 試験: 新サイズを要求 → Resized → relayout。
                    let target = (1020u32, 700u32);
                    s.resize_target = Some(target);
                    window.set_inner_size(LogicalSize::new(target.0, target.1));
                    s.stage = 3;
                    s.stage_ticks = 0;
                }
            }
            // stage 3: resize 追従の確認（Resized 反映 + JS viewport + frames 継続）。
            3 => {
                let Some((tw, th)) = s.resize_target else {
                    s.stage = 4;
                    return;
                };
                let win_ok = s.resize_events.iter().any(|&(w2, h2)| {
                    (w2 as i32 - tw as i32).abs() <= 4 && (h2 as i32 - th as i32).abs() <= 4
                });
                let expect_left = ((tw as f64) * WEBVIEW_FRACTION) as u32;
                let js_ok = s
                    .js_resizes
                    .iter()
                    .any(|&(jw, _)| (jw as i32 - expect_left as i32).abs() <= 8);
                if (win_ok && js_ok) || s.stage_ticks > 60 {
                    s.record(
                        "resize_follow",
                        if win_ok && js_ok {
                            Check::pass(format!(
                                "window {tw}x{th} reached; js viewport -> left {expect_left}px"
                            ))
                        } else {
                            Check::fail(format!(
                                "win_ok={win_ok} js_ok={js_ok} resizes={:?} js={:?}",
                                s.resize_events, s.js_resizes
                            ))
                        },
                    );
                    s.stage = 4;
                    s.stage_ticks = 0;
                }
            }
            // stage 4: z-order オーバーラップ試験（adjacent-child のみ意味を持つ）。
            4 => {
                if adjacent_child {
                    let sb = sibling.borrow();
                    let res = match sb.as_ref() {
                        Some(c) => {
                            // sibling を WebView 領域（左）へ一時移動し兄弟 z-order
                            // 最上位へ raise → 重なり中央を覆う自ツリー最上位 HWND を
                            // 照会（初期配置では webview コンテナ側が上位に留まる
                            // 観測があり、明示 raise の可否を確認する）。
                            c.move_to(0, 0, lw, h);
                            c.bring_to_top();
                            let cov = covering_children(parent_hwnd, lw / 2, h / 2);
                            let wfp = hwnd_at(parent_hwnd, lw / 2, h / 2);
                            let top = cov.first();
                            let pass = top
                                .map(|(hc, _)| hwnd_matches(*hc, c.hwnd()))
                                .unwrap_or(false);
                            let note = format!(
                                "sibling moved over webview region; top covering {top:?} \
                                 (sibling {:?}; wfp={wfp:?})",
                                c.hwnd()
                            );
                            // 元の右領域へ戻す。
                            let (x, y, cw, ch) = right_rect_px(w as u32, h as u32);
                            c.move_to(x, y, cw, ch);
                            if pass {
                                Check::pass(note)
                            } else {
                                Check::fail(note)
                            }
                        }
                        None => Check::fail("sibling HWND not created"),
                    };
                    s.record("z_order_overlap", res);
                } else {
                    s.record(
                        "z_order_overlap",
                        Check::obs("adjacent-parent: no sibling HWND (webview child sits above parent surface)"),
                    );
                }
                // SetWindowRgn 試行: WebView 子 HWND の下端に切り欠きを作り、
                // その内側で自ツリーの被覆が消えるか・native に抜けるかを照会する。
                let notch = (lw / 4, h - 120, lw / 2, h - 20);
                let clip_res = clip_hwnd(wv, Some((0, 0, lw, h - 120)));
                let f = hwnd_at(
                    parent_hwnd,
                    (notch.0 + notch.2) / 2,
                    (notch.1 + notch.3) / 2,
                );
                match clip_res {
                    Ok(()) => {
                        // notch 内側で WebView 系の被覆が消えたかを自ツリーで照会。
                        let cov = covering_children(
                            parent_hwnd,
                            (notch.0 + notch.2) / 2,
                            (notch.1 + notch.3) / 2,
                        );
                        let webview_still_covers = cov.iter().any(|(hc, c)| {
                            hwnd_matches(*hc, wv) || c.starts_with("Chrome_WidgetWin")
                        });
                        let wfp_hit_native = f.map(|hf| hwnd_matches(hf, nat)).unwrap_or(false);
                        s.clip_note = Some(format!(
                            "clip(0,0,{lw},{}) notch {notch:?}: covering={cov:?} wfp -> {f:?} (native={nat:?})",
                            h - 120,
                        ));
                        s.record(
                            "region_clip",
                            if !webview_still_covers || wfp_hit_native {
                                Check::pass("clip removed webview coverage at notch")
                            } else {
                                Check::obs(
                                    "clip applied; webview subtree still covers notch (child HWNDs not region-clipped)",
                                )
                            },
                        );
                        let _ = clip_hwnd(wv, None); // 復元
                    }
                    Err(e) => {
                        s.clip_note = Some(format!("clip failed: {e}"));
                        s.record("region_clip", Check::fail(format!("SetWindowRgn: {e}")));
                    }
                }
                s.stage = 5;
                s.stage_ticks = 0;
            }
            // stage 5: フレーム継続の最終確認（resize 後も present が続いているか）。
            5 if s.stage_ticks >= 8 => {
                s.record(
                    "frame_continuity",
                    if s.frames >= 40 {
                        Check::pass(format!(
                            "{} frames presented across resize/z-order/clip",
                            s.frames
                        ))
                    } else {
                        Check::fail(format!("only {} frames", s.frames))
                    },
                );
                s.done = true;
                s.exit_reason = Some("matrix_complete");
            }
            _ => {}
        }
    }

    /// 終了時のメトリック組立と status 判定。
    fn finish_metrics(s: &S, mode: &str) -> Outcome {
        let matrix: serde_json::Map<String, Value> = s
            .matrix
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
        let missing: Vec<&'static str> = [
            "surface_init",
            "webview_child",
            "hit_test_webview",
            "hit_test_native",
            "focus_transition",
            "z_order_overlap",
            "resize_follow",
            "frame_continuity",
            "region_clip",
        ]
        .iter()
        .copied()
        .filter(|k| !s.has(k))
        .collect();

        let metrics = json!({
            "mode": mode,
            "matrix": matrix,
            "frames_presented": s.frames,
            "frames_skipped": s.frames_skipped,
            "frame_errors": s.frame_errors,
            "webview_hwnd": s.webview_hwnd.map(|h| format!("{h:?}")),
            "native_hwnd": s.native_hwnd.map(|h| format!("{h:?}")),
            "children": s.children,
            "resize_events": s.resize_events,
            "js_resize_tail": s.js_resizes,
            "focus_events": s.focus_events,
            "clip_note": s.clip_note,
            "exit_reason": s.exit_reason,
            "timeout_hit": s.timeout_hit,
            "errors": s.errors,
        });

        // 実験としての成立判定: 全検査項目が（pass/fail/obs いずれかで）
        // 記録され、フレームが 1 枚以上 present されていれば passed。
        // 個別項目の fail は採否材料として残し実験自体は失敗としない
        // （記録要件 vs SLA の区別 — test-spec 正本）。
        let status = if !missing.is_empty() {
            Status::Failed(format!("matrix incomplete: missing {missing:?}"))
        } else if s.frames == 0 {
            Status::Failed("no wgpu frames presented".to_string())
        } else {
            Status::Passed
        };
        Outcome { metrics, status }
    }
}

/// 非 Windows: 実験自体を前提不足扱い（本 PoC は Windows 実機検証）。
#[cfg(not(windows))]
mod win {
    use crate::args::RunArgs;
    use crate::metrics::Status;
    use serde_json::json;

    use super::Outcome;

    pub fn run(mode: &str, _args: &RunArgs) -> Outcome {
        Outcome {
            metrics: json!({"mode": mode}),
            status: Status::Skipped("surface experiment is Windows-only".to_string()),
        }
    }
}
