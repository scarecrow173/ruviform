//! WebView ホスト（adapter 層）。wry / tao / http 等の外部型はこのファイルと
//! `assets.rs`・`check.rs` に封じ、`exp/*` はここが定義する内部 I/F
//! （`HostEvent`・`Host`）にのみ依存する（design「将来クレート分割の予行」）。
//!
//! 構成: tao `Window` + wry `WebView`（`build` + `set_bounds` 追従）。
//! `build_as_child` 経路は本機 WebView2 155 で子コンテナのリサイズが
//! JS viewport に反映されない実測のため採用しない（T4/AC3 の検証論点）。
//! 全イベントは `HostEvent` に正規化して実験コールバックへ転送し、実験は
//! `Host` の最小操作 I/F（eval / resize / focus / finish）だけを使う。
//!
//! 自己終了契約: `config.timeout` 到達で `HostEvent::Timeout` を 1 度だけ
//! 配信したあとイベントループを必ず終了する（オペレータ入力を無限に待たない）。

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

use crate::assets;

/// ホスト生成パラメータ。
pub struct HostConfig {
    /// 初期ロード URL（`poc://app/index.html` 等）。
    pub url: String,
    /// ウィンドウタイトル。
    pub title: String,
    /// 初期クライアント領域サイズ（論理 px）。
    pub width: u32,
    /// 初期クライアント領域高さ（論理 px）。
    pub height: u32,
    /// 自己終了タイムアウト。到達で Timeout → ループ終了。
    pub timeout: Duration,
    /// WebView2 データディレクトリ。None は一時 dir（再現性のため既定は一時領域）。
    pub data_dir: Option<PathBuf>,
}

/// 実験へ転送する正規化イベント（wry/tao 型を含まない内部 I/F）。
pub enum HostEvent {
    /// ページロード完了（`PageLoadEvent::Finished`）。`url` は最終 URL。
    PageLoadFinished { url: String },
    /// `window.ipc.postMessage` が発行した JSON 文字列（`Request<String>` の body）。
    Ipc(String),
    /// ウィンドウリサイズ（物理 px）。webview `set_bounds` は host 側で適用済み。
    Resized { w: u32, h: u32 },
    /// tao ウィンドウのフォーカス遷移。
    Focused(bool),
    /// クローズ要求（実験側が finish しなくてもループは終了する）。
    CloseRequested,
    /// `timeout` 到達（1 度だけ配信・配信後ループは必ず終了）。
    Timeout,
}

/// 実験がホストへ行える最小操作。メソッドは失敗しても panic しない
/// （`Result` は捨てる。PoC では観測失敗はメトリック欠落として現れる）。
pub struct Host<'a> {
    webview: &'a WebView,
    window: &'a tao::window::Window,
    want_exit: &'a Rc<Cell<bool>>,
}

impl Host<'_> {
    /// `evaluate_script` による JS 評価（Rust → JS push 経路）。
    pub fn eval(&self, js: &str) {
        let r = self.webview.evaluate_script(js);
        eprintln!("[host] eval({js}) -> {r:?}");
    }

    /// ウィンドウのクライアント領域サイズ変更を要求する（→ Resized → set_bounds 追従）。
    pub fn resize_window(&self, w: u32, h: u32) {
        self.window.set_inner_size(LogicalSize::new(w, h));
    }

    /// 現在のクライアント領域サイズ（物理 px）。
    pub fn inner_size(&self) -> (u32, u32) {
        let s = self.window.inner_size();
        (s.width, s.height)
    }

    /// webview bounds を現在のクライアント領域へ再適用する（冪等）。
    /// `jitter` px を幅に加える。WebView2 の `controller.SetBounds` は
    /// 非同期反映のため、同一値の再送が合体/破棄されるケースへの
    /// リトライで ±1px を交互に与えて確実な差分を作る用途に使う。
    pub fn refresh_bounds(&self, jitter: i32) -> Result<(), String> {
        let s = self.window.inner_size();
        let w = (s.width as i32 + jitter).max(1) as u32;
        self.webview
            .set_bounds(full_rect(w, s.height))
            .map_err(|e| e.to_string())
    }

    /// WebView 側フォーカス制御。`true` で `focus()`（`MoveFocus`）、
    /// `false` で `focus_parent()`（`SetFocus(parent)`）。
    /// 成功可否を `Result<(), String>` で返す（wry 型を境界外へ漏らさない）。
    pub fn focus_webview(&self, focused: bool) -> Result<(), String> {
        let r = if focused {
            self.webview.focus()
        } else {
            self.webview.focus_parent()
        };
        eprintln!("[host] focus_webview({focused}) -> {r:?}");
        r.map_err(|e| e.to_string())
    }

    /// tao ウィンドウ自体へ OS フォーカスを要求する（`Window::set_focus`）。
    /// 新規ウィンドウが foreground activation を得られていない環境でも
    /// フォーカス遷移を観測するための起点として使う。
    pub fn focus_window(&self) {
        self.window.set_focus();
    }

    /// 実験完了シグナル。次のイベント処理後にイベントループを終了する。
    pub fn finish(&self) {
        self.want_exit.set(true);
    }
}

/// 物理 px の全領域 Rect（webview をクライアント領域全体に敷く）。
fn full_rect(w: u32, h: u32) -> Rect {
    Rect {
        position: Position::Physical(PhysicalPosition::new(0, 0)),
        size: Size::Physical(PhysicalSize::new(w.max(1), h.max(1))),
    }
}

/// イベントループを構築して `on_event` にイベントを配信する。
/// `run_return` はループ終了後に戻るため、呼び出し側はメトリック集計を続行できる。
/// ウィンドウ/WebView 生成に失敗した場合のみ Err を返す（実験失敗扱い）。
pub fn run<F>(config: &HostConfig, mut on_event: F) -> Result<(), String>
where
    F: FnMut(HostEvent, &Host<'_>),
{
    // user event 対応の EventLoop: WebView2 callback（IPC/ページロード）は
    // OS ウィンドウイベントとして表れず `WaitUntil` を起床させないため、
    // `create_proxy().send_event(())` で起床を駆動する。
    let mut event_loop: EventLoop<()> = EventLoopBuilder::<()>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(true)
        .build(&event_loop)
        .map_err(|e| format!("window build failed: {e}"))?;

    // wry/tao のイベントは UI スレッド上で同期的に処理されるため、
    // handler → イベントループ間の受け渡しは単一スレッドの Rc キューで足りる。
    // ただし WebView2 の IPC / ページロード callback は tao の `WaitUntil`
    // スリープを起床させないため、イベントループ代理（EventLoopProxy）の
    // user event で起床を明示する。これがないと、ウィンドウイベントが途絶えた
    // 区間に積まれた IPC イベント（done シグナル等）は deadline まで
    // 配信されない（T1 の timeout_hit 併記と同根因。render 実験で顕在化）。
    let queue: Rc<RefCell<VecDeque<HostEvent>>> = Rc::new(RefCell::new(VecDeque::new()));
    let proxy = event_loop.create_proxy();

    // WebView2 データディレクトリは一時 dir 配下のプロセス固有パスとする。
    // WebView2 は UDF にシングルトンロックを持ち、前回実行のブラウザ子プロセスが
    // 生き残っていると次回起動の WebView2 初期化がブロックされる（連続実行で
    // 実測）。固定パスによる再現性より、実行間の隔離を優先する。
    let data_dir = config.data_dir.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("ui-poc-webview2-{}", std::process::id()))
    });
    let mut web_context = WebContext::new(Some(data_dir));

    let initial = window.inner_size();
    let webview = {
        let q_ipc = Rc::clone(&queue);
        let q_load = Rc::clone(&queue);
        let proxy_ipc = proxy.clone();
        let proxy_load = proxy.clone();
        WebViewBuilder::new_with_web_context(&mut web_context)
            .with_url(&config.url)
            .with_custom_protocol("poc".to_string(), |_id, request| {
                assets::handle_poc_protocol(&request)
            })
            .with_ipc_handler(move |request| {
                q_ipc
                    .borrow_mut()
                    .push_back(HostEvent::Ipc(request.body().clone()));
                // UI スレッド上の callback 内からでもループを起床させる
                // （Windows では proxy がイベントループの隠しウィンドウへ PostMessage）。
                let _ = proxy_ipc.send_event(());
            })
            .with_on_page_load_handler(move |event, url| {
                if matches!(event, PageLoadEvent::Finished) {
                    q_load
                        .borrow_mut()
                        .push_back(HostEvent::PageLoadFinished { url });
                    let _ = proxy_load.send_event(());
                }
            })
            .with_bounds(full_rect(initial.width, initial.height))
            // `build`（非子ビュー）: Windows では wry が親 HWND に subclass を
            // 付けて WM_SIZE → `controller.SetBounds` の自動リサイズを行う。
            // 備考: `build_as_child` でも同じ `set_bounds_inner` 経路だが、
            // 本機 WebView2 155 では子コンテナ resize 後も viewport が
            // 追従しない観測となったため host 実験は build 経路を採用した
            // （子ビュー bounds 追従は T4/AC3 の検証論点として引き継ぐ）。
            .build(&window)
            .map_err(|e| format!("webview build failed: {e}"))?
    };

    let deadline = Instant::now() + config.timeout;
    // コールバック側（Host::finish）とループ側で共有する終了フラグ。
    // ループ側が clone を move で持つため、Host が参照する元は Rc に置く。
    let want_exit = Rc::new(Cell::new(false));
    let loop_exit = Rc::clone(&want_exit);
    let timeout_sent = Cell::new(false);
    let host = Host {
        webview: &webview,
        window: &window,
        want_exit: &want_exit,
    };

    event_loop.run_return(move |event, _target, control_flow| {
        // handler が積んだイベント（ipc/page load）を先に消化する。
        loop {
            let next = queue.borrow_mut().pop_front();
            match next {
                Some(ev) => on_event(ev, &host),
                None => break,
            }
        }
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                // タイムアウトは 1 度だけ通知し、その後は必ず終了する（自己終了契約）。
                if !timeout_sent.replace(true) {
                    on_event(HostEvent::Timeout, &host);
                }
                loop_exit.set(true);
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // `set_bounds` の明示呼出（リサイズ追従 API の検証対象）。
                // build 経路では wry の親 subclass が既に SetBounds 済みだが、
                // 冪等なため同一呼出は無害。
                if let Err(e) = host.webview.set_bounds(full_rect(size.width, size.height)) {
                    eprintln!("[host] set_bounds failed: {e}");
                }
                eprintln!(
                    "[host] resized {}x{} -> webview bounds {:?}",
                    size.width,
                    size.height,
                    host.webview.bounds()
                );
                on_event(
                    HostEvent::Resized {
                        w: size.width,
                        h: size.height,
                    },
                    &host,
                );
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } => {
                on_event(HostEvent::Focused(focused), &host);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                on_event(HostEvent::CloseRequested, &host);
                loop_exit.set(true);
            }
            _ => {}
        }
        // キュー消化中に追加されたイベントを取りこぼさない。
        loop {
            let next = queue.borrow_mut().pop_front();
            match next {
                Some(ev) => on_event(ev, &host),
                None => break,
            }
        }

        if loop_exit.get() {
            *control_flow = ControlFlow::Exit;
        } else {
            *control_flow = ControlFlow::WaitUntil(deadline);
        }
    });
    Ok(())
}
