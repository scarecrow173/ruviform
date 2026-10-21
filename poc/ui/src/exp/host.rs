//! `--experiment host`（AC1: WebView ホスト）。
//!
//! 検証項目（test-spec 正本）:
//! - `protocol_load`: `poc://app/index.html` が埋め込みアセットからロードされる
//!   （`PageLoadFinished` + JS `hello` の両確認。dev server 不使用）。
//! - `ipc_echo`: JS `window.ipc.postMessage` JSON intent → `with_ipc_handler`
//!   受理 → `evaluate_script` echo 返却の往復成立と RTT（ms）。
//! - `resize_follow`: ウィンドウリサイズ → `set_bounds` 追従 → JS
//!   ResizeObserver が要求サイズを観測すること。
//! - `focus_transition`: `focus()`/`focus_parent()` 起点のフォーカス遷移が
//!   JS window focus/blur（`mechanism:"window"`）または DOM 要素プローブ
//!   （`mechanism:"dom_element"` フォールバック）で観測されること。
//!   tao `Focused` は OS foreground activation 依存のため別証跡として記録する。
//! - `webview2_version`: `check.rs` のレジストリ検出値。
//!
//! 状態機械は Rust 側が所有し、JS は観測イベントの報告のみ行う。
//! `done` メッセージ・全項目収集・`Timeout` のいずれかで終了し、
//! 未収集項目があれば `failed`（`passed` には全項目 ok が必要）。

use std::cell::RefCell;
use std::time::Duration;

use serde_json::{Value, json};

use crate::args::RunArgs;
use crate::assets;
use crate::check::Environment;
use crate::host::{self, HostConfig, HostEvent};
use crate::metrics::Status;

use super::{Experiment, Outcome, Skip};

/// AC1 実験。状態は `run` 内に閉じる（実験間の相互干渉を持たない）。
pub struct HostExperiment;

impl HostExperiment {
    /// 実験インスタンス生成（レジストリから呼ばれる）。
    pub fn new() -> Self {
        Self
    }
}

/// 実験フェーズ（Rust 所有の状態機械）。
#[derive(Debug)]
enum Phase {
    /// ページロード完了待ち。
    AwaitLoad,
    /// JS `hello`（スクリプト実行証明）→ `ping` 受信待ち。
    AwaitPing,
    /// `evaluate_script` echo 返却 → JS `ipc_echo` 受信待ち。
    AwaitEcho,
    /// リサイズ要求済み → JS `resize_observed`（要求サイズ一致）待ち。
    AwaitResize,
    /// focus トグル済み → JS focus イベント観測待ち。
    AwaitFocus,
    /// 収集完了。
    Done,
}

#[derive(Default)]
struct State {
    phase: Option<Phase>,
    loaded_url: Option<String>,
    hello_seen: bool,
    ipc_echo: Option<Value>,
    resize_requested: Option<(u32, u32)>,
    resize_follow: Option<Value>,
    /// tao 側が観測した最終ウィンドウサイズ（resize_follow の対側証跡）。
    last_window_size: Option<(u32, u32)>,
    /// JS `resize_observed` の生観測列（不一致診断用。先頭8件まで）。
    resize_samples: Vec<[i64; 2]>,
    /// AwaitResize 中の bounds 再適用リトライ回数（ハートビート駆動）。
    resize_retries: u8,
    /// JS viewport ハートビートの最新値（診断）。
    viewport: Option<Value>,
    js_focus_events: Vec<bool>,
    tao_focus_events: Vec<bool>,
    /// `focus()`/`focus_parent()` API 呼出結果（["ok"|"err:..."]）。
    api_focus: Vec<String>,
    /// DOM 要素フォーカスプローブの観測イベント列（dom_focus フォールバック）。
    dom_events: Vec<String>,
    /// プローブ時点の `document.hasFocus()` 真値。
    dom_has_focus: Option<bool>,
    /// DOM フォールバック（`focusCycle` eval）を投入済みか。
    dom_fallback_sent: bool,
    timeout_hit: bool,
    got_done: bool,
    /// ループ終了の直接要因（done_signal/metrics_complete/timeout/close_requested）。
    exit_reason: Option<&'static str>,
    errors: Vec<String>,
}

impl Experiment for HostExperiment {
    fn name(&self) -> &'static str {
        "host"
    }

    fn setup(&mut self, env: &Environment) -> Result<(), Skip> {
        // 前提不足は status:"skipped"（test-spec のゲーティング契約）。
        if env.headless_env {
            return Err(Skip("UI_POC_HEADLESS=1".to_string()));
        }
        if !env.display_available {
            return Err(Skip("no display environment".to_string()));
        }
        if cfg!(windows) && env.webview2_version.is_none() {
            return Err(Skip("WebView2 runtime not detected".to_string()));
        }
        if !assets::frontend_built() {
            return Err(Skip(
                "frontend not built: run `npm ci && npm run build` in poc/ui/frontend".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&mut self, env: &Environment, args: &RunArgs) -> Outcome {
        let timeout = Duration::from_secs(args.duration_secs.unwrap_or(20));
        let config = HostConfig {
            url: "poc://app/index.html".to_string(),
            title: "ui-poc host (AC1)".to_string(),
            width: 900,
            height: 600,
            timeout,
            data_dir: None,
        };

        let st = RefCell::new(State {
            phase: Some(Phase::AwaitLoad),
            ..State::default()
        });

        let result = host::run(&config, |event, host| {
            let mut s = st.borrow_mut();
            dispatch(event, host, &mut s);
        });

        if let Err(e) = result {
            return Outcome {
                metrics: json!({}),
                status: Status::Failed(format!("host run failed: {e}")),
            };
        }

        let s = st.into_inner();
        let metrics = assemble_metrics(&s, env);
        let status = evaluate(&s);
        Outcome { metrics, status }
    }
}

/// 単一イベントの状態機械遷移。
fn dispatch(event: HostEvent, host: &host::Host<'_>, s: &mut State) {
    match event {
        HostEvent::PageLoadFinished { url } => {
            if s.loaded_url.is_none() {
                s.loaded_url = Some(url);
                s.phase = Some(Phase::AwaitPing);
            }
        }
        HostEvent::Ipc(body) => handle_ipc(&body, host, s),
        HostEvent::Resized { w, h } => {
            // set_bounds は host 層で適用済み。tao 側観測を証跡として残し、
            // JS 側の resize_observed を待つ。
            s.last_window_size = Some((w, h));
        }
        HostEvent::Focused(focused) => {
            s.tao_focus_events.push(focused);
        }
        HostEvent::CloseRequested => {
            // 実験中のクローズ要求はそのまま終了（収集済みで評価）。
            s.exit_reason = Some("close_requested");
        }
        HostEvent::Timeout => {
            s.timeout_hit = true;
            s.exit_reason = Some("timeout");
            host.finish();
        }
    }
}

/// JS intent（`window.ipc.postMessage` の JSON 文字列）の処理。
fn handle_ipc(body: &str, host: &host::Host<'_>, s: &mut State) {
    let msg: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            s.errors.push(format!("ipc parse error: {e}"));
            return;
        }
    };
    match msg.get("type").and_then(Value::as_str) {
        Some("hello") => {
            s.hello_seen = true;
        }
        Some("ping") => {
            // echo 応答はフェーズ非依存・冪等: ipc/ping が PageLoadFinished より
            // 先に届く配送順があり得るため、未記録なら常に応答する
            // （postMessage 可能な時点で evaluate_script も機能する）。
            if s.ipc_echo.is_none() {
                let t0 = msg.get("t0").cloned().unwrap_or(json!(0));
                s.phase = Some(Phase::AwaitEcho);
                // evaluate_script で echo を返す（ipc_echo RTT の復路）。
                let payload = serde_json::to_string(&json!({"t0": t0, "seq": 1}))
                    .unwrap_or_else(|_| "{}".to_string());
                host.eval(&format!("window.__poc && window.__poc.echo({payload})"));
            }
        }
        Some("ipc_echo") => {
            if s.ipc_echo.is_none() {
                let ok = msg.get("ok").and_then(Value::as_bool).unwrap_or(false);
                let rtt = msg.get("rtt_ms").and_then(Value::as_f64).unwrap_or(0.0);
                s.ipc_echo = Some(json!({"ok": ok, "rtt_ms": rtt}));
                // リサイズ追従の検証: 初期サイズから非対称に変更し、
                // JS ResizeObserver の観測値と突き合わせる。
                let (w0, h0) = host.inner_size();
                let req = (w0 + 47, h0 + 31);
                s.resize_requested = Some(req);
                s.phase = Some(Phase::AwaitResize);
                host.resize_window(req.0, req.1);
            }
        }
        Some("resize_observed") => {
            let ow = msg.get("w").and_then(Value::as_i64).unwrap_or(-1);
            let oh = msg.get("h").and_then(Value::as_i64).unwrap_or(-1);
            if s.resize_samples.len() < 8 {
                s.resize_samples.push([ow, oh]);
            }
            // 要求済み・未記録の場合のみ突き合わせ（配送順非依存・冪等）。
            if s.resize_follow.is_none()
                && let Some((rw, rh)) = s.resize_requested
            {
                // DPI 丸め・スクロールバーを考慮した ±4px 許容。
                let matched = (ow - i64::from(rw)).abs() <= 4 && (oh - i64::from(rh)).abs() <= 4;
                if matched {
                    s.resize_follow = Some(json!({
                        "ok": true,
                        "requested": [rw, rh],
                        "observed": [ow, oh],
                        "retries": s.resize_retries,
                    }));
                    begin_focus_phase(host, s);
                }
            }
        }
        Some("viewport") => {
            // 診断ハートビート: 最新の JS viewport を記録する。
            let w = msg.get("w").and_then(Value::as_i64).unwrap_or(-1);
            let h = msg.get("h").and_then(Value::as_i64).unwrap_or(-1);
            let dpr = msg.get("dpr").and_then(Value::as_f64).unwrap_or(0.0);
            s.viewport = Some(json!({
                "w": w,
                "h": h,
                "dpr": dpr,
                "hasFocus": msg.get("hasFocus").and_then(Value::as_bool),
                "visibility": msg.get("visibility").and_then(Value::as_str),
            }));
            // ハートビート駆動の周期処理（約2s間隔）。
            match s.phase {
                Some(Phase::AwaitResize) if s.resize_follow.is_none() => {
                    // リサイズ反映の取りこぼし対策。
                    // WebView2 `controller.SetBounds` は非同期反映であり、
                    // 初期レイアウト完了前後の同一値更新が反映されない
                    // ケースを実測で観測した。±1px の交互ジッタを加えて
                    // 「毎回必ず異なる値」の bounds 再適用を行い、
                    // 確実な差分としてブラウザへ伝播させる。
                    if s.resize_retries < 4 {
                        s.resize_retries += 1;
                        let jitter = if s.resize_retries % 2 == 1 { -1 } else { 0 };
                        let _ = host.refresh_bounds(jitter);
                    } else {
                        // リトライ上限: resize_follow を失敗確定として
                        // focus 段階へ進行し、残りの証跡も収集する。
                        begin_focus_phase(host, s);
                    }
                }
                // OS 経路での window-level 遷移が観測できないまま待機中なら
                // DOM 要素フォーカスプローブへフォールバックする（1 回のみ）。
                Some(Phase::AwaitFocus) if !focus_ok(s) && !s.dom_fallback_sent => {
                    s.dom_fallback_sent = true;
                    host.eval("window.__poc && window.__poc.focusCycle()");
                }
                _ => {}
            }
        }
        Some("focus") => {
            if let Some(f) = msg.get("focused").and_then(Value::as_bool) {
                s.js_focus_events.push(f);
            }
            if focus_ok(s) && matches!(s.phase, Some(Phase::AwaitFocus)) {
                s.phase = Some(Phase::Done);
                s.exit_reason = Some("metrics_complete");
                host.finish();
            }
        }
        Some("dom_focus") => {
            // DOM 要素フォーカスプローブの観測結果（フォールバック経路）。
            s.dom_events = msg
                .get("events")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            s.dom_has_focus = msg.get("hasFocus").and_then(Value::as_bool);
            if focus_ok(s) && matches!(s.phase, Some(Phase::AwaitFocus)) {
                s.phase = Some(Phase::Done);
                s.exit_reason = Some("metrics_complete");
                host.finish();
            }
        }
        Some("done") => {
            s.got_done = true;
            s.exit_reason = Some("done_signal");
            host.finish();
        }
        Some(_) | None => {
            s.errors.push(format!("unknown ipc message: {body}"));
        }
    }
}

/// フォーカス遷移フェーズへ進行する（AwaitResize 成立時 or リトライ上限時の共通経路）。
/// 1) OS 経路: tao `set_focus`（SetForegroundWindow）→
///    wry `focus_parent`（SetFocus(parent)）→ wry `focus`（MoveFocus）。
///    API 結果を証跡として記録する。
/// 2) OS フォーカスが得られない環境（バックグラウンド起動等）では
///    window-level 遷移が観測されないため、次の viewport ハートビート
///    到達時に DOM 要素プローブ（`focusCycle`）へフォールバックする。
fn begin_focus_phase(host: &host::Host<'_>, s: &mut State) {
    // AwaitResize からの遷移のみ許可（遅到着の resize_observed 一致や
    // リトライ上限到達が AwaitFocus/Done 後でも再シーケンスしないようにする）。
    // resize_follow の記録自体はフェーズ非依存で行われるため、ここで
    // 早期リターンしても証跡は失われない。
    if !matches!(s.phase, Some(Phase::AwaitResize)) {
        return;
    }
    s.phase = Some(Phase::AwaitFocus);
    host.focus_window();
    s.api_focus.push("set_focus:invoked".to_string());
    for focused in [false, true] {
        let api = if focused { "focus" } else { "focus_parent" };
        let r = host
            .focus_webview(focused)
            .map(|_| "ok".to_string())
            .unwrap_or_else(|e| format!("err:{e}"));
        s.api_focus.push(format!("{api}:{r}"));
    }
    // 既に十分な遷移が観測済みなら即終了（新規イベント待ちでタイムアウトまで消費しない）。
    if focus_ok(s) {
        s.phase = Some(Phase::Done);
        s.exit_reason = Some("metrics_complete");
        host.finish();
    }
}

/// フォーカス遷移の観測メカニズム同定。
/// - `"window"`: window-level 遷移（JS blur→focus ペア、tao Focused ペア、
///   または JS/tao 両側 1 件以上）。OS フォーカスが実際に観測できた最強の証跡。
/// - `"dom_element"`: DOM 要素 focus→blur プローブ観測。OS フォーカス非依存の
///   フォールバック（`evaluate_script` 往復と DOM フォーカス機構の実証を兼ねる）。
/// - `"none"`: 未観測。
fn focus_mechanism(s: &State) -> &'static str {
    let js_pair = s.js_focus_events.windows(2).any(|w| w == [false, true]);
    let tao_pair = s.tao_focus_events.windows(2).any(|w| w == [false, true]);
    if js_pair || tao_pair || (!s.js_focus_events.is_empty() && !s.tao_focus_events.is_empty()) {
        return "window";
    }
    let has_focus = s.dom_events.iter().any(|e| e == "focus");
    let has_blur = s.dom_events.iter().any(|e| e == "blur");
    if has_focus && has_blur {
        "dom_element"
    } else {
        "none"
    }
}

/// フォーカス遷移の成立判定（いずれかのメカニズムで遷移が観測されたか）。
fn focus_ok(s: &State) -> bool {
    focus_mechanism(s) != "none"
}

/// 収集済み状態から `poc-metric.v1` の `metrics` を組み立てる。
fn assemble_metrics(s: &State, env: &Environment) -> Value {
    let mechanism = focus_mechanism(s);
    let focus_note = match mechanism {
        "window" => Value::Null,
        "dom_element" => json!(
            "OS-level window focus not observable (no foreground activation); \
             DOM element focus transition verified via evaluate_script probe"
        ),
        _ => json!("no focus transition observed"),
    };
    json!({
        "protocol_load": {
            "ok": s.loaded_url.is_some() && s.hello_seen,
            "url": s.loaded_url,
            "hello_seen": s.hello_seen,
        },
        "ipc_echo": s.ipc_echo.clone().unwrap_or_else(|| json!({"ok": false})),
        "resize_follow": s.resize_follow.clone().unwrap_or_else(|| {
            json!({
                "ok": false,
                "requested": s.resize_requested.map(|(w,h)| [w,h]),
                "window_size": s.last_window_size.map(|(w,h)| [w,h]),
                "observed_samples": s.resize_samples,
                "retries": s.resize_retries,
            })
        }),
        "focus_transition": {
            "ok": mechanism != "none",
            "mechanism": mechanism,
            "api": s.api_focus,
            "js_window_events": s.js_focus_events,
            "tao_events": s.tao_focus_events,
            "dom_events": s.dom_events,
            "document_hasFocus": s.dom_has_focus,
            "note": focus_note,
        },
        "webview2_version": env.webview2_version,
        "timeout_hit": s.timeout_hit,
        "done_signal": s.got_done,
        "exit_reason": s.exit_reason,
        "viewport": s.viewport,
        "errors": s.errors,
    })
}

/// 終了評価。全 AC1 項目が成立して `passed`、それ以外は `failed`。
fn evaluate(s: &State) -> Status {
    let mut missing: Vec<&str> = Vec::new();
    if !(s.loaded_url.is_some() && s.hello_seen) {
        missing.push("protocol_load");
    }
    if s.ipc_echo.is_none() {
        missing.push("ipc_echo");
    }
    if s.resize_follow.is_none() {
        missing.push("resize_follow");
    }
    if !focus_ok(s) {
        missing.push("focus_transition");
    }
    if missing.is_empty() {
        Status::Passed
    } else {
        Status::Failed(format!(
            "incomplete metrics (missing: {}){}",
            missing.join(","),
            if s.timeout_hit { ", timeout" } else { "" }
        ))
    }
}
