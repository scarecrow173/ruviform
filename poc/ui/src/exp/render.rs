//! `--experiment render`（AC2: WebView 内描画バックエンド計測）。
//!
//! 検証項目（design/test-spec 正本）:
//! - 同一ワークロード（スクロール波形・ピーク列 約2,000点/フレーム +
//!   レベルメータ群、60fps 目安）を `poc://app/vanilla/index.html` の
//!   bench UI で `--backend canvas2d|webgl2|webgpu` に応じて描画する。
//! - JS 側 rAF タイムスタンプ差分列（`frame_time_ms`）とフレーム内作業時間
//!   （`work_time_ms`）を done メッセージで受領し、`metrics::summarize` が
//!   `avg/p50/p95/p99` を集約。`dropped`（> 2 vsync 間隔）と `achieved_fps`
//!   は vsync 推定が必要なため JS 側で算出済みの値を記録する。
//! - WebGPU は `navigator.gpu` + adapter 要求の機能検出失敗時に WebGL2 へ
//!   フォールバックし、`status:"passed"` + `metrics.fallback_to` に理由を
//!   記録する（フォールバック自体は失敗としない）。
//!
//! 状態機械は Rust 側が所有し、JS は `ready`/`alive`/`done`/`bench_error`
//! の報告のみ行う。`--frames`（既定 600）サンプル記録で自己終了し、
//! `default_timeout_secs` の timeout でも必ず終了する。

use std::cell::RefCell;
use std::time::Duration;

use serde_json::{Value, json};

use crate::args::RunArgs;
use crate::assets;
use crate::check::Environment;
use crate::host::{self, HostConfig, HostEvent};
use crate::metrics::{self, Status};

use super::{Experiment, Outcome, Skip, check_gui_prereqs};

/// AC2 実験。状態は `run` 内に閉じる。
pub struct RenderExperiment;

/// `--backend` が取りうる値（args 検証と JS cfg の共用正本）。
const BACKENDS: [&str; 3] = ["canvas2d", "webgl2", "webgpu"];

/// `alive` heartbeat ベースの start 再送上限。
const MAX_START_RETRIES: u8 = 3;

impl RenderExperiment {
    /// 実験インスタンス生成（レジストリから呼ばれる）。
    pub fn new() -> Self {
        Self
    }
}

/// フレーム数からの既定タイムアウト（秒）。
/// 60fps 理想でも warmup・検出・環境揺れで実時間は伸びうるため、
/// 理想時間の 4 倍 + 固定 20s（最低 30s）を与える。--duration で上書き可能。
fn default_timeout_secs(frames: u64) -> u64 {
    (frames / 15 + 20).max(30)
}

/// 実験フェーズ（Rust 所有の状態機械）。
#[derive(Debug)]
enum Phase {
    /// ページロード完了待ち。
    AwaitLoad,
    /// JS `ready` 通知待ち（受信で `__poc.start` を eval 送信）。
    AwaitReady,
    /// ベンチ実行中（`done`/`bench_error`/timeout 待ち）。
    Running,
    /// 収集完了。
    Done,
}

struct State {
    phase: Option<Phase>,
    /// 要求バックエンド（検証済み値）。
    backend: String,
    /// 要求フレーム数（JS へ渡す記録サンプル数）。
    frames: u64,
    loaded_url: Option<String>,
    ready_seen: bool,
    start_sent: bool,
    /// alive( idle ) 駆動の start 再送回数。
    start_retries: u8,
    /// `alive` ハートビートの受信総数と末尾サンプル（診断用）。
    alive_count: u64,
    alive_tail: Vec<Value>,
    /// JS `done` の `stats` フィールド（生値）。
    done_stats: Option<Value>,
    /// JS `bench_error`（レンダラ生成不能等の致命的失敗）。
    bench_error: Option<String>,
    timeout_hit: bool,
    got_done: bool,
    /// ループ終了の直接要因（done_signal/bench_error/timeout/close_requested）。
    exit_reason: Option<&'static str>,
    errors: Vec<String>,
}

impl Experiment for RenderExperiment {
    fn name(&self) -> &'static str {
        "render"
    }

    fn setup(&mut self, env: &Environment) -> Result<(), Skip> {
        check_gui_prereqs(env)?;
        // 部分ビルド（marker あり・variant 欠落）も前提不足として skip する。
        if !assets::app_asset_exists("vanilla/index.html") {
            return Err(Skip(
                "frontend not built: vanilla variant dist missing \
                 (run `npm ci && npm run build` in poc/ui/frontend)"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn run(&mut self, env: &Environment, args: &RunArgs) -> Outcome {
        // --backend は必須。欠落/不正値は実験失敗として記録する
        // （CLI エラー exit 2 ではなく、パラメータ起因の失敗として扱う）。
        let backend = match args.backend.as_deref() {
            Some(b) if BACKENDS.contains(&b) => b.to_string(),
            Some(other) => {
                return Outcome {
                    metrics: json!({"backend": other}),
                    status: Status::Failed(format!(
                        "invalid --backend '{other}' (expected canvas2d|webgl2|webgpu)"
                    )),
                };
            }
            None => {
                return Outcome {
                    metrics: json!({}),
                    status: Status::Failed(
                        "missing --backend (expected canvas2d|webgl2|webgpu)".to_string(),
                    ),
                };
            }
        };

        let frames = args.frames;
        let timeout = Duration::from_secs(
            args.duration_secs
                .unwrap_or_else(|| default_timeout_secs(frames)),
        );
        let config = HostConfig {
            url: "poc://app/vanilla/index.html".to_string(),
            title: format!("ui-poc render (AC2 {backend})"),
            width: 960,
            height: 640,
            timeout,
            tick: None,
            data_dir: None,
        };

        let st = RefCell::new(State {
            phase: Some(Phase::AwaitLoad),
            backend,
            frames,
            loaded_url: None,
            ready_seen: false,
            start_sent: false,
            start_retries: 0,
            alive_count: 0,
            alive_tail: Vec::new(),
            done_stats: None,
            bench_error: None,
            timeout_hit: false,
            got_done: false,
            exit_reason: None,
            errors: Vec::new(),
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
                s.phase = Some(Phase::AwaitReady);
                // ready より先に start を投げても JS 側 start は冪等。
                // __poc 未定義なら eval は無視され、ready 受信時に再送される。
                send_start(host, s);
            }
        }
        HostEvent::Ipc(body) => handle_ipc(&body, host, s),
        HostEvent::Resized { .. } => {
            // ベンチは固定サイズで計測する（リサイズ中は計測値が揺れる）。
            // resize 検証は AC1 の責務。ここでは証跡のみ残す余地として無視する。
        }
        HostEvent::Focused(_) => {}
        HostEvent::CloseRequested => {
            s.exit_reason = Some("close_requested");
        }
        HostEvent::Timeout => {
            s.timeout_hit = true;
            s.exit_reason = Some("timeout");
            host.finish();
        }
        // tick 非利用実験では配信されない（HostConfig::tick=None）。
        HostEvent::Tick => {}
    }
}

/// `__poc.start({backend, frames})` を evaluate_script で送信する。
/// JS 側 start は実行中/完了後の呼出を無視する冪等設計のため重複送信は無害。
fn send_start(host: &host::Host<'_>, s: &mut State) {
    let cfg = serde_json::to_string(&json!({
        "backend": s.backend,
        "frames": s.frames,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    host.eval(&format!("window.__poc && window.__poc.start({cfg})"));
    s.start_sent = true;
    s.phase = Some(Phase::Running);
}

/// JS intent の処理。
fn handle_ipc(body: &str, host: &host::Host<'_>, s: &mut State) {
    let msg: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            s.errors.push(format!("ipc parse error: {e}"));
            return;
        }
    };
    match msg.get("type").and_then(Value::as_str) {
        Some("ready") => {
            s.ready_seen = true;
            if s.done_stats.is_none() {
                send_start(host, s);
            }
        }
        Some("alive") => {
            // liveness 証跡（末尾8件 + 総数）。
            s.alive_count += 1;
            if s.alive_tail.len() >= 8 {
                s.alive_tail.remove(0);
            }
            s.alive_tail.push(msg.clone());
            // start が JS に届いていない（__poc 未定義タイミングの eval 喪失等）
            // とき alive.state=="idle" が返る。bounded に再送する。
            let js_state = msg.get("state").and_then(Value::as_str);
            if js_state == Some("idle")
                && s.done_stats.is_none()
                && s.bench_error.is_none()
                && s.start_retries < MAX_START_RETRIES
            {
                s.start_retries += 1;
                send_start(host, s);
            }
        }
        Some("bench_error") => {
            let m = msg
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown bench error")
                .to_string();
            s.bench_error = Some(m);
            s.phase = Some(Phase::Done);
            s.exit_reason = Some("bench_error");
            host.finish();
        }
        Some("done") => {
            s.done_stats = msg.get("stats").cloned();
            s.got_done = true;
            s.phase = Some(Phase::Done);
            s.exit_reason = Some("done_signal");
            host.finish();
        }
        Some(_) | None => {
            s.errors.push(format!("unknown ipc message: {body}"));
        }
    }
}

/// done.stats のサンプル列を f64 ベクタへ変換する。
fn f64_vec(v: Option<&Value>) -> Vec<f64> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

/// 収集済み状態から `poc-metric.v1` の `metrics` を組み立てる。
/// `frame_time_ms`/`work_time_ms` の percentile 集計は metrics.rs が担う契約。
fn assemble_metrics(s: &State, env: &Environment) -> Value {
    let stats = s.done_stats.clone().unwrap_or_else(|| json!({}));
    let frame_samples = f64_vec(stats.get("frame_time_ms"));
    let work_samples = f64_vec(stats.get("work_time_ms"));
    json!({
        "backend": s.backend,
        "backend_used": stats.get("backend_used"),
        "fallback_to": stats.get("fallback"),
        "capabilities": stats.get("caps"),
        "frames": stats.get("frames"),
        "frames_requested": s.frames,
        "warmup_frames": stats.get("warmup_frames"),
        "elapsed_ms": stats.get("elapsed_ms"),
        "vsync_ms": stats.get("vsync_ms"),
        "frame_time_ms": metrics::summarize(&frame_samples),
        "work_time_ms": metrics::summarize(&work_samples),
        "dropped": stats.get("dropped"),
        "achieved_fps": stats.get("achieved_fps"),
        "protocol_load": s.loaded_url.is_some(),
        "ready_seen": s.ready_seen,
        "start_retries": s.start_retries,
        "alive": {"count": s.alive_count, "tail": s.alive_tail},
        "bench_error": s.bench_error,
        "timeout_hit": s.timeout_hit,
        "done_signal": s.got_done,
        "exit_reason": s.exit_reason,
        "errors": s.errors,
        "webview2_version": env.webview2_version,
    })
}

/// 終了評価。
/// - done.stats に 1 フレーム以上の記録があれば `passed`（fallback 込みでも可）。
/// - `bench_error` は実験失敗。
/// - done 未到達（timeout・close）は実験失敗（記録要件未充足）。
fn evaluate(s: &State) -> Status {
    if let Some(err) = &s.bench_error {
        return Status::Failed(format!("bench error: {err}"));
    }
    match &s.done_stats {
        Some(stats) if stats.get("frames").and_then(Value::as_u64).unwrap_or(0) > 0 => {
            Status::Passed
        }
        Some(_) => Status::Failed("done received but zero frames recorded".to_string()),
        None => Status::Failed(format!(
            "no done stats{} (ready:{} start_sent:{} retries:{} alive:{})",
            if s.timeout_hit { ", timeout" } else { "" },
            s.ready_seen,
            s.start_sent,
            s.start_retries,
            s.alive_count
        )),
    }
}
