//! `--experiment ipc`（AC4: メッセージ境界計測）。
//!
//! 検証項目（design/test-spec 正本）— Rust ⇄ フロントエンド間の 3 経路:
//! - **JSON RTT**: JS `{type:"rtt_ping",seq,t0,pad}` → Rust が `pong(seq)` を
//!   `evaluate_script` 返送 → JS が `t1-t0` を計測。ペイロード
//!   64B/1KiB/16KiB/64KiB × `RTT_ITERS` 回 → `ipc_rtt_ms{avg,p50,p99}`。
//! - **単方向 ingest**: JS が `ingest` メッセージを `INGEST_N` 件フラッド →
//!   `ingest_end` までの Rust 側到達窓から `ipc_ingest_msg_s`（msg/s）。
//! - **バイナリ pull**: JS が `fetch("poc://data/blob?bytes=N")` を反復 →
//!   `protocol_pull_MBps`（64KiB/1MiB/16MiB）+ リクエスト毎レイテンシ。
//! - **push**: Rust が `evaluate_script` をバースト（`PUSH_BURST` 連続）+
//!   tick 定速（`PUSH_TICK` 間隔 × `PUSH_PACED` 件）で送信 → JS 側の
//!   受信間隔・ドロップ・coalesce を `push_msg_s` + 観察として記録。
//! - **coexist**: push 送信中に rtt ping + pull fetch を並行実行し、
//!   3 経路併用の実現性証跡を採る。
//!
//! 状態機械は Rust 側が所有し、JS は `ready`/`alive`/`done`/`bench_error` と
//! 測定駆動メッセージ（`rtt_ping`/`ingest`/`push_begin`/`coexist_begin`）を
//! 報告する。percentile 集計は `metrics::summarize`（render と同じ分担）。

use std::cell::RefCell;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::args::RunArgs;
use crate::assets;
use crate::check::Environment;
use crate::host::{self, HostConfig, HostEvent};
use crate::metrics::{self, Status};

use super::{Experiment, Outcome, Skip, check_gui_prereqs};

/// AC4 実験。状態は `run` 内に閉じる。
pub struct IpcExperiment;

/// JSON RTT のペイロードサイズ系列（JSON メッセージ全体の目標バイト数）。
const RTT_SIZES: [u32; 4] = [64, 1024, 16384, 65536];
/// サイズあたりの RTT 反復回数。
const RTT_ITERS: u32 = 32;
/// 単方向 ingest フラッド件数。
const INGEST_N: u32 = 500;
/// バイナリ pull のサイズ系列。
const PULL_SIZES: [u32; 3] = [65536, 1048576, 16777216];
/// pull サイズごとの反復回数（大サイズは回数を絞り時間枠に収める）。
const PULL_ITERS: [u32; 3] = [16, 8, 3];
/// push バースト送信件数（`evaluate_script` 連続呼出のストレス観察）。
const PUSH_BURST: u32 = 200;
/// push 定速送信件数（メインフェーズ）。
const PUSH_PACED: u32 = 160;
/// push 定速送信件数（coexist フェーズ）。
const PUSH_COEXIST: u32 = 96;
/// 定速送信の tick 間隔（名目 250 msg/s 目標。実効は OS タイマ分解能依存）。
const PUSH_TICK: Duration = Duration::from_millis(4);
/// coexist フェーズの push seq 起点（メイン push の seq 帯 1..=360 と分離）。
const COEXIST_SEQ_BASE: u32 = 1000;
/// coexist フェーズで並行する rtt ping 回数（64B 固定）。
const COEXIST_RTT_ITERS: u32 = 8;

/// `alive` heartbeat ベースの start 再送上限（render と同じ規約）。
const MAX_START_RETRIES: u8 = 3;

impl IpcExperiment {
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
    /// JS `ready` 通知待ち（受信で `__poc.start` を eval 送信）。
    AwaitReady,
    /// ベンチ実行中（`done`/`bench_error`/timeout 待ち）。
    Running,
    /// 収集完了。
    Done,
}

/// tick 駆動の定速 push 送信計画。
struct Pace {
    /// 次に送る seq（`next_seq` → `next_seq + total - 1` まで）。
    next_seq: u32,
    /// 送信総数。
    total: u32,
    /// 送信済み数。
    sent: u32,
    /// 証跡用ラベル（"paced" / "coexist"）。
    label: &'static str,
}

struct State {
    phase: Option<Phase>,
    loaded_url: Option<String>,
    ready_seen: bool,
    start_sent: bool,
    /// alive( idle ) 駆動の start 再送回数。
    start_retries: u8,
    /// `alive` ハートビートの受信総数と末尾サンプル（診断用）。
    alive_count: u64,
    alive_tail: Vec<Value>,
    // ---- ingest（Rust 側到達レート）----
    ingest_first: Option<Instant>,
    ingest_last: Option<Instant>,
    ingest_count: u64,
    ingest_end_seen: bool,
    // ---- push ----
    pace: Option<Pace>,
    push_sent_burst: u32,
    push_sent_paced: u32,
    push_sent_coexist: u32,
    /// 実配信された tick 数と最初/最後の時刻（実効間隔の証跡）。
    tick_count: u64,
    tick_first: Option<Instant>,
    tick_last: Option<Instant>,
    /// JS `done` の `stats` フィールド（生値）。
    done_stats: Option<Value>,
    bench_error: Option<String>,
    timeout_hit: bool,
    got_done: bool,
    /// ループ終了の直接要因（done_signal/bench_error/timeout/close_requested）。
    exit_reason: Option<&'static str>,
    errors: Vec<String>,
}

impl Experiment for IpcExperiment {
    fn name(&self) -> &'static str {
        "ipc"
    }

    fn setup(&mut self, env: &Environment) -> Result<(), Skip> {
        check_gui_prereqs(env)?;
        // 部分ビルド（marker あり・ipc variant 欠落）も前提不足として skip。
        if !assets::app_asset_exists("ipc/index.html") {
            return Err(Skip(
                "frontend not built: ipc variant dist missing \
                 (run `npm ci && npm run build` in poc/ui/frontend)"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn run(&mut self, env: &Environment, args: &RunArgs) -> Outcome {
        // 全フェーズ合計 ~6s 見込み。既定 30s を上限とし --duration で上書き。
        let timeout = Duration::from_secs(args.duration_secs.unwrap_or(30));
        let config = HostConfig {
            url: "poc://app/ipc/index.html".to_string(),
            title: "ui-poc ipc (AC4)".to_string(),
            width: 800,
            height: 520,
            timeout,
            tick: Some(PUSH_TICK),
            data_dir: None,
        };

        let st = RefCell::new(State {
            phase: Some(Phase::AwaitLoad),
            loaded_url: None,
            ready_seen: false,
            start_sent: false,
            start_retries: 0,
            alive_count: 0,
            alive_tail: Vec::new(),
            ingest_first: None,
            ingest_last: None,
            ingest_count: 0,
            ingest_end_seen: false,
            pace: None,
            push_sent_burst: 0,
            push_sent_paced: 0,
            push_sent_coexist: 0,
            tick_count: 0,
            tick_first: None,
            tick_last: None,
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
                // ready より先に start を投げても JS 側 start は冪等
                // （render と同じ起動規約）。
                send_start(host, s);
            }
        }
        HostEvent::Ipc(body) => handle_ipc(&body, host, s),
        HostEvent::Resized { .. } | HostEvent::Focused(_) => {}
        HostEvent::CloseRequested => {
            s.exit_reason = Some("close_requested");
        }
        HostEvent::Timeout => {
            s.timeout_hit = true;
            s.exit_reason = Some("timeout");
            host.finish();
        }
        HostEvent::Tick => {
            s.tick_count += 1;
            let now = Instant::now();
            if s.tick_first.is_none() {
                s.tick_first = Some(now);
            }
            s.tick_last = Some(now);
            if s.pace.is_some() {
                send_paced_push(host, s);
            }
        }
    }
}

/// `__poc.start(cfg)` を evaluate_script で送信する。
/// 反復回数・サイズ系列は Rust 側が正本として注入する（計測契約の単一源泉）。
fn send_start(host: &host::Host<'_>, s: &mut State) {
    let cfg = serde_json::to_string(&json!({
        "rtt_sizes": RTT_SIZES,
        "rtt_iters": RTT_ITERS,
        "ingest_n": INGEST_N,
        "pull_sizes": PULL_SIZES,
        "pull_iters": PULL_ITERS,
        "push_burst": PUSH_BURST,
        "push_paced": PUSH_PACED,
        "push_coexist": PUSH_COEXIST,
        "push_tick_ms": PUSH_TICK.as_millis(),
        "coexist_seq_base": COEXIST_SEQ_BASE,
        "coexist_rtt_iters": COEXIST_RTT_ITERS,
        "coexist_pull": [[65536, 4], [1048576, 2]],
        "stall_ms": 2500,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    host.eval(&format!("window.__poc && window.__poc.start({cfg})"));
    s.start_sent = true;
    s.phase = Some(Phase::Running);
}

/// `__poc.push(seq)` の定速送信（Tick 駆動）。1 tick あたり 1 件送り、
/// 送り切ったら `__poc.pushDone()` で JS 側の受信待ちを解放する。
/// 送信側でバッチ化しないことで、「間隔を空けて送った push が受信側で
/// まとめて届くか」という coalesce 観察を汚さない設計。
fn send_paced_push(host: &host::Host<'_>, s: &mut State) {
    let Some(p) = &mut s.pace else { return };
    if p.sent < p.total {
        let seq = p.next_seq + p.sent;
        host.eval_quiet(&format!("window.__poc && window.__poc.push({seq})"));
        p.sent += 1;
        match p.label {
            "coexist" => s.push_sent_coexist += 1,
            _ => s.push_sent_paced += 1,
        }
    }
    if p.sent >= p.total {
        host.eval_quiet("window.__poc && window.__poc.pushDone()");
        s.pace = None;
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
        Some("ready") => {
            s.ready_seen = true;
            if s.done_stats.is_none() {
                send_start(host, s);
            }
        }
        Some("alive") => {
            s.alive_count += 1;
            if s.alive_tail.len() >= 8 {
                s.alive_tail.remove(0);
            }
            s.alive_tail.push(msg.clone());
            // start 取りこぼし時の再送（render と同じ bounded リトライ）。
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
        Some("rtt_ping") => {
            // JSON RTT 経路の復路: seq を evaluate_script で即返送する。
            // 高頻度パスのため eval_quiet（eprintln コストを計測から除く）。
            let seq = msg.get("seq").and_then(Value::as_i64).unwrap_or(-1);
            host.eval_quiet(&format!("window.__poc && window.__poc.pong({seq})"));
        }
        Some("ingest") => {
            // 単方向フラッド: 到達時刻を記録（アプリ側に見える取り込みレート）。
            let now = Instant::now();
            if s.ingest_first.is_none() {
                s.ingest_first = Some(now);
            }
            s.ingest_last = Some(now);
            s.ingest_count += 1;
        }
        Some("ingest_end") => {
            s.ingest_end_seen = true;
        }
        Some("push_begin") => {
            // push 経路: まず evaluate_script をバースト連続送信し、
            // 続きを tick 定速で送る（バースト/定速の両挙動を観察）。
            if s.pace.is_none() {
                for seq in 1..=PUSH_BURST {
                    host.eval_quiet(&format!("window.__poc && window.__poc.push({seq})"));
                }
                s.push_sent_burst += PUSH_BURST;
                s.pace = Some(Pace {
                    next_seq: PUSH_BURST + 1,
                    total: PUSH_PACED,
                    sent: 0,
                    label: "paced",
                });
            } else {
                s.errors
                    .push("push_begin received while pace already armed".to_string());
            }
        }
        Some("coexist_begin") => {
            // 併用フェーズ: push 送信中に JS が rtt/ping と fetch/pull を並行する。
            if s.pace.is_none() {
                s.pace = Some(Pace {
                    next_seq: COEXIST_SEQ_BASE + 1,
                    total: PUSH_COEXIST,
                    sent: 0,
                    label: "coexist",
                });
            } else {
                s.errors
                    .push("coexist_begin received while pace already armed".to_string());
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

/// done.stats のサンプル列を f64 ベクタへ変換する（render と同じ規約）。
fn f64_vec(v: Option<&Value>) -> Vec<f64> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

/// `{"64":[..], "1024":[..]}` 形のサイズ別サンプル列をサイズ別 summarize へ変換。
fn summarize_by_size(v: Option<&Value>) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = v.and_then(Value::as_object) {
        for (k, arr) in obj {
            out.insert(k.clone(), metrics::summarize(&f64_vec(Some(arr))));
        }
    }
    Value::Object(out)
}

/// pull 統計 `{bytes, iters, latencies}` からスループット（MB/s = 1e6 bytes/s）。
fn pull_mbps(size_bytes: f64, iters: f64, lat_sum_ms: f64) -> f64 {
    if lat_sum_ms > 0.0 {
        size_bytes * iters / (lat_sum_ms / 1000.0) / 1.0e6
    } else {
        0.0
    }
}

/// 収集済み状態から `poc-metric.v1` の `metrics` を組み立てる。
fn assemble_metrics(s: &State, env: &Environment) -> Value {
    let stats = s.done_stats.clone().unwrap_or_else(|| json!({}));
    let rtt_obj = stats.get("rtt");
    let pull_obj = stats.get("pull");

    // バイナリ pull: サイズ別にレイテンシ集計 + 合計時間から MB/s。
    let mut pull_mbps_map = serde_json::Map::new();
    let mut pull_lat_map = serde_json::Map::new();
    if let Some(obj) = pull_obj.and_then(Value::as_object) {
        for (k, rec) in obj {
            let lats = f64_vec(rec.get("latencies"));
            let bytes = rec.get("bytes").and_then(Value::as_f64).unwrap_or(0.0);
            let iters = rec.get("iters").and_then(Value::as_f64).unwrap_or(0.0);
            pull_lat_map.insert(k.clone(), metrics::summarize(&lats));
            pull_mbps_map.insert(k.clone(), json!(pull_mbps(bytes, iters, lats.iter().sum())));
        }
    }

    // ingest: Rust 側到達窓から単方向 msg/s。
    let ingest_window_ms = match (s.ingest_first, s.ingest_last) {
        (Some(f), Some(l)) => (l - f).as_secs_f64() * 1000.0,
        _ => 0.0,
    };
    let ingest_msg_s = if ingest_window_ms > 0.0 && s.ingest_count > 1 {
        (s.ingest_count - 1) as f64 / (ingest_window_ms / 1000.0)
    } else {
        0.0
    };

    // tick の実効間隔（定速 push の実送信レート証跡）。
    let tick_avg_ms = match (s.tick_first, s.tick_last) {
        (Some(f), Some(l)) if s.tick_count > 1 => {
            (l - f).as_secs_f64() * 1000.0 / (s.tick_count - 1) as f64
        }
        _ => 0.0,
    };

    json!({
        // ---- AC4 必須メトリック ----
        "ipc_rtt_ms": summarize_by_size(rtt_obj),
        "ipc_rtt_missed": stats.get("rtt_missed"),
        "ipc_ingest_msg_s": ingest_msg_s,
        "ipc_ingest": {
            "sent_declared": stats.get("ingest").and_then(|v| v.get("sent")),
            "received": s.ingest_count,
            "window_ms": ingest_window_ms,
            "end_seen": s.ingest_end_seen,
        },
        "protocol_pull_MBps": Value::Object(pull_mbps_map),
        "protocol_pull_latency_ms": Value::Object(pull_lat_map),
        "push_msg_s": stats.get("push").and_then(|v| v.get("msg_s")),
        "push": {
            "sent_burst": s.push_sent_burst,
            "sent_paced": s.push_sent_paced,
            "sent_coexist": s.push_sent_coexist,
            "sent_total": s.push_sent_burst + s.push_sent_paced + s.push_sent_coexist,
            "received": stats.get("push").and_then(|v| v.get("received")),
            "dropped": stats.get("push").and_then(|v| v.get("dropped")),
            "coalesced": stats.get("push").and_then(|v| v.get("coalesced")),
            "max_batch": stats.get("push").and_then(|v| v.get("max_batch")),
            "elapsed_ms": stats.get("push").and_then(|v| v.get("elapsed_ms")),
            "interval_ms": metrics::summarize(&f64_vec(
                stats.get("push").and_then(|v| v.get("interval_ms")),
            )),
            "target_interval_ms": PUSH_TICK.as_millis(),
            "tick_count": s.tick_count,
            "tick_avg_ms": tick_avg_ms,
        },
        "coexist": {
            "rtt_ms": metrics::summarize(&f64_vec(
                stats.get("coexist").and_then(|v| v.get("rtt_ms")),
            )),
            "rtt_missed": stats.get("coexist").and_then(|v| v.get("rtt_missed")),
            "pull_ms": summarize_by_size(
                stats.get("coexist").and_then(|v| v.get("pull_ms")),
            ),
            "push_received": stats.get("coexist").and_then(|v| v.get("push_received")),
            "push_dropped": stats.get("coexist").and_then(|v| v.get("push_dropped")),
            "push": stats.get("coexist").and_then(|v| v.get("push")),
        },
        "bench_cfg": stats.get("cfg_echo"),
        "phases_ms": stats.get("phases_ms"),
        // ---- ハウスキーピング ----
        "protocol_load": s.loaded_url.is_some(),
        "ready_seen": s.ready_seen,
        "start_retries": s.start_retries,
        "alive": {"count": s.alive_count, "tail": s.alive_tail},
        "bench_error": s.bench_error,
        "js_errors": stats.get("errors"),
        "timeout_hit": s.timeout_hit,
        "done_signal": s.got_done,
        "exit_reason": s.exit_reason,
        "errors": s.errors,
        "webview2_version": env.webview2_version,
    })
}

/// 必須記録項目が揃っているかの判定。
/// - done.stats に rtt（全 4 サイズ）・pull（全 3 サイズ）・push 受信があれば `passed`。
/// - `bench_error` は実験失敗。done 未到達（timeout・close）は実験失敗。
fn evaluate(s: &State) -> Status {
    if let Some(err) = &s.bench_error {
        return Status::Failed(format!("bench error: {err}"));
    }
    let Some(stats) = &s.done_stats else {
        return Status::Failed(format!(
            "no done stats{} (ready:{} start_sent:{} retries:{} alive:{})",
            if s.timeout_hit { ", timeout" } else { "" },
            s.ready_seen,
            s.start_sent,
            s.start_retries,
            s.alive_count
        ));
    };
    let mut missing: Vec<&str> = Vec::new();
    let rtt = stats.get("rtt").and_then(Value::as_object);
    for size in RTT_SIZES {
        let ok = rtt
            .and_then(|m| m.get(&size.to_string()))
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
        if !ok {
            missing.push("ipc_rtt_ms");
            break;
        }
    }
    let pull = stats.get("pull").and_then(Value::as_object);
    for size in PULL_SIZES {
        let ok = pull
            .and_then(|m| m.get(&size.to_string()))
            .and_then(|r| r.get("latencies"))
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
        if !ok {
            missing.push("protocol_pull_MBps");
            break;
        }
    }
    if s.ingest_count < 2 || !s.ingest_end_seen {
        missing.push("ipc_ingest_msg_s");
    }
    let push_received = stats
        .get("push")
        .and_then(|v| v.get("received"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if push_received == 0 {
        missing.push("push_msg_s");
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
