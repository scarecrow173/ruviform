//! `--experiment frontend --variant vanilla|solid`（AC5: フロントエンド FW 比較）。
//!
//! 検証項目（design/test-spec 正本）:
//! - 同一ベンチ UI（AC2 ワークロード + 同一 DOM 構造 + bench-shared 計装）を
//!   `poc://app/<variant>/index.html` で実行し、FW 差を数値化する。
//! - 記録:
//!   - `dist` 合計バイト（`bytes_raw` と `bytes_gzip9` = ファイル毎の
//!     gzip -9（flate2/miniz_oxide `Compression::best()`）合計。
//!     `build-meta.json` は計測メタデータであり vite 産物ではないため除外）。
//!     再現コマンドは RESULTS.md 併記。
//!   - Vite ビルド時間（build.mjs が dist/<variant>/build-meta.json に刻印した
//!     `build_ms` — 実験実行時のビルドではなくワークフロー側の実測値）。
//!   - glue-code LOC（`frontend/<variant>/{index.html,vite.config.ts,src/**}`
//!     の行数・非空行数。FS のソースを読む開発時計測）。
//!   - 更新粒度（bench 期間中の DOM MutationRecord 集計
//!     `dom_mutations` と `dom_mutations_per_meter_update` — Solid の
//!     fine-grained 更新が部分適用に留まるか）。
//!   - 開発ワークフロー所見は RESULTS.md / impl log に記述。
//! - `bench_error`・done 未到達は実験失敗。variant dist 欠落は `skipped`
//!   （前提不足。run 内判定 — setup は args を見ないため）。
//!
//! 状態機械は Rust 側が所有し、JS は `ready`/`alive`/`done`/`bench_error`
//! の報告のみ行う（render 実験と同じ規約）。

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use crate::args::RunArgs;
use crate::assets;
use crate::check::Environment;
use crate::host::{self, HostConfig, HostEvent};
use crate::metrics::{self, Status};

use super::{Experiment, Outcome, Skip, check_gui_prereqs};

/// AC5 実験。状態は `run` 内に閉じる。
pub struct FrontendExperiment;

/// `--variant` が取りうる値（args 検証と資産パスの共用正本）。
const VARIANTS: [&str; 2] = ["vanilla", "solid"];

/// `--backend` が取りうる値（render と同じ正本。既定 canvas2d で両 variant を揃える）。
const BACKENDS: [&str; 3] = ["canvas2d", "webgl2", "webgpu"];

/// dist バイト集計から除外する計測メタデータ名（build.mjs が刻印する）。
/// vite の産物ではないため「dist 合計バイト」に含めない契約。
const BUILD_META_NAME: &str = "build-meta.json";

/// `alive` heartbeat ベースの start 再送上限（render と同じ規約）。
const MAX_START_RETRIES: u8 = 3;

impl FrontendExperiment {
    /// 実験インスタンス生成（レジストリから呼ばれる）。
    pub fn new() -> Self {
        Self
    }
}

/// フレーム数からの既定タイムアウト（render と同じ式）。
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
    /// JS `bench_error`（致命的失敗）。
    bench_error: Option<String>,
    timeout_hit: bool,
    got_done: bool,
    /// ループ終了の直接要因（done_signal/bench_error/timeout/close_requested）。
    exit_reason: Option<&'static str>,
    errors: Vec<String>,
}

impl Experiment for FrontendExperiment {
    fn name(&self) -> &'static str {
        "frontend"
    }

    fn setup(&mut self, env: &Environment) -> Result<(), Skip> {
        // 共通 GUI 前提（headless・display・WebView2・frontend ビルド済みマーカー）。
        // variant 別 dist 欠落の検査は args 依存のため run 側で行う。
        check_gui_prereqs(env)
    }

    fn run(&mut self, env: &Environment, args: &RunArgs) -> Outcome {
        // --variant は必須。欠落/不正値は実験失敗として記録する
        // （render の --backend と同じ規約）。
        let variant = match args.variant.as_deref() {
            Some(v) if VARIANTS.contains(&v) => v.to_string(),
            Some(other) => {
                return Outcome {
                    metrics: json!({"variant": other}),
                    status: Status::Failed(format!(
                        "invalid --variant '{other}' (expected vanilla|solid)"
                    )),
                };
            }
            None => {
                return Outcome {
                    metrics: json!({}),
                    status: Status::Failed(
                        "missing --variant (expected vanilla|solid)".to_string(),
                    ),
                };
            }
        };

        // variant 別 dist が無い部分ビルドは前提不足 → skipped（非空 reason）。
        // setup は args を見られないため、ここで判定する。
        if !assets::app_asset_exists(&format!("{variant}/index.html")) {
            return Outcome {
                metrics: json!({"variant": variant}),
                status: Status::Skipped(format!(
                    "frontend not built: {variant} variant dist missing \
                     (run `npm ci && npm run build` in poc/ui/frontend)"
                )),
            };
        }

        // --backend は省略時 canvas2d（両 variant の計測条件を揃える既定）。
        let backend = match args.backend.as_deref() {
            None => "canvas2d".to_string(),
            Some(b) if BACKENDS.contains(&b) => b.to_string(),
            Some(other) => {
                return Outcome {
                    metrics: json!({"variant": variant, "backend": other}),
                    status: Status::Failed(format!(
                        "invalid --backend '{other}' (expected canvas2d|webgl2|webgpu)"
                    )),
                };
            }
        };

        // FS/埋め込み側メトリックは bench 成否と独立して収集する
        // （失敗時にも dist バイト・LOC の証跡を残す）。
        let dist_metrics = variant_dist_metrics(&variant);
        let loc_metrics = glue_loc(&variant);

        let frames = args.frames;
        let timeout = Duration::from_secs(
            args.duration_secs
                .unwrap_or_else(|| default_timeout_secs(frames)),
        );
        let config = HostConfig {
            url: format!("poc://app/{variant}/index.html"),
            title: format!("ui-poc frontend (AC5 {variant})"),
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
                metrics: json!({
                    "variant": variant,
                    "dist": dist_metrics,
                    "glue_loc": loc_metrics,
                }),
                status: Status::Failed(format!("host run failed: {e}")),
            };
        }

        let s = st.into_inner();
        let metrics = assemble_metrics(&s, &variant, dist_metrics, loc_metrics, env);
        let status = evaluate(&s);
        Outcome { metrics, status }
    }
}

/// 単一イベントの状態機械遷移（render と同じ規約）。
fn dispatch(event: HostEvent, host: &host::Host<'_>, s: &mut State) {
    match event {
        HostEvent::PageLoadFinished { url } => {
            if s.loaded_url.is_none() {
                s.loaded_url = Some(url);
                s.phase = Some(Phase::AwaitReady);
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
        HostEvent::Tick => {}
    }
}

/// `__poc.start({backend, frames})` を evaluate_script で送信する（冪等）。
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

/// JS intent の処理（render と同じ規約）。
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

/// done.stats のサンプル列を f64 ベクタへ変換する（render と同じ規約）。
fn f64_vec(v: Option<&Value>) -> Vec<f64> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

/// 1 ファイルの gzip -9（`Compression::best()`）圧縮後バイト数。
/// flate2 の既定 backend は pure-Rust miniz_oxide（依存追加の理由・実装名を
/// `gzip9_impl` メトリックに記録し、GNU gzip との実装差を透明にする）。
fn gzip9_len(data: &[u8]) -> usize {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    if enc.write_all(data).is_err() {
        return 0;
    }
    enc.finish().map(|v| v.len()).unwrap_or(0)
}

/// `<variant>/` 配下のアセット集計（release=埋め込み辞書・debug=FS 列挙）。
/// `bytes_raw` はファイルサイズ合計、`bytes_gzip9` はファイル毎 gzip -9 の合計
/// （HTTP gzip 転送コストの近似モデル）。`build-meta.json` は計測メタデータの
/// ため集計から除外し、その内容は `build_meta` として別枠記録する。
fn variant_dist_metrics(variant: &str) -> Value {
    let prefix = format!("{variant}/");
    let mut files: Vec<Value> = Vec::new();
    let mut bytes_raw: u64 = 0;
    let mut bytes_gzip9: u64 = 0;
    let mut names: Vec<String> = assets::app_asset_names()
        .into_iter()
        .filter(|n| n.starts_with(&prefix))
        .collect();
    names.sort();
    for name in names {
        let rel = &name[prefix.len()..];
        if rel == BUILD_META_NAME {
            continue;
        }
        let Some(data) = assets::app_asset_data(&name) else {
            continue;
        };
        let gz = gzip9_len(&data);
        bytes_raw += data.len() as u64;
        bytes_gzip9 += gz as u64;
        files.push(json!({"name": rel, "bytes": data.len(), "gzip9": gz}));
    }

    // build.mjs が刻印するワークフロー側メタ（Vite ビルド時間・FS 側バイト内訳）。
    let meta = assets::app_asset_data(&format!("{variant}/{BUILD_META_NAME}"))
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok());
    let meta_total = meta.as_ref().and_then(|m| m.get("total_bytes")).cloned();
    // 埋め込み配信物と FS 側 dist が一致するかの自己検証
    // （meta.total_bytes は build.mjs が FS の vite 産物から計算した値）。
    let meta_total_match = meta_total
        .as_ref()
        .and_then(Value::as_u64)
        .map(|t| t == bytes_raw);

    json!({
        "bytes_raw": bytes_raw,
        "bytes_gzip9": bytes_gzip9,
        "gzip9_impl": "flate2 (miniz_oxide) Compression::best() level 9, per-file sum",
        "gzip9_scope": format!("embedded poc://app/{variant}/ assets (excludes {BUILD_META_NAME})"),
        "files": files,
        "build_meta": meta,
        "meta_total_bytes_match": meta_total_match,
    })
}

/// glue-code LOC 集計。`frontend/<variant>/` の `{index.html, vite.config.ts,
/// src/**}` を対象に行数・非空行数を数える（FS ソースを読む開発時計測。
/// バイナリ単体コピーでは frontend/ が存在せず null を返す）。
fn glue_loc(variant: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("frontend")
        .join(variant);
    if !root.is_dir() {
        return Value::Null;
    }
    let mut targets: Vec<PathBuf> = Vec::new();
    for top in ["index.html", "vite.config.ts"] {
        let p = root.join(top);
        if p.is_file() {
            targets.push(p);
        }
    }
    let src = root.join("src");
    if src.is_dir() {
        collect_src_files(&src, &mut targets);
    }
    targets.sort();

    let mut files = Vec::new();
    let mut total_lines: u64 = 0;
    let mut total_nonblank: u64 = 0;
    for path in targets {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines = content.lines().count() as u64;
        let nonblank = content.lines().filter(|l| !l.trim().is_empty()).count() as u64;
        total_lines += lines;
        total_nonblank += nonblank;
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(json!({"path": rel, "lines": lines, "nonblank": nonblank}));
    }
    json!({
        "scope": format!("frontend/{variant}/{{index.html,vite.config.ts,src/**}}"),
        "files": files,
        "total_lines": total_lines,
        "total_nonblank": total_nonblank,
    })
}

/// `src/` 以下のソースファイルを再帰列挙（順序は後でソート）。
fn collect_src_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_src_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// 収集済み状態から `poc-metric.v1` の `metrics` を組み立てる。
fn assemble_metrics(
    s: &State,
    variant: &str,
    dist: Value,
    glue_loc: Value,
    env: &Environment,
) -> Value {
    let stats = s.done_stats.clone().unwrap_or_else(|| json!({}));
    let frame_samples = f64_vec(stats.get("frame_time_ms"));
    let work_samples = f64_vec(stats.get("work_time_ms"));
    let dom = stats.get("dom_mutations");
    // 1 メータ更新当たりの attribute mutation 数。
    // attributes / (meter_channels × observed_frames)。vanilla は setLevels が
    // 無条件で 8 fill へ書くため ≒1.0 が下限参照。Solid の fine-grained 更新が
    // 同じ部分適用粒度に留まるか、childList churn を出さないかが論点。
    let per_meter = dom.and_then(|d| {
        let attrs = d.get("attributes")?.as_f64()?;
        let ch = d.get("meter_channels")?.as_f64()?;
        let frames = d.get("observed_frames")?.as_f64()?;
        (ch * frames > 0.0).then(|| attrs / (ch * frames))
    });
    json!({
        // ---- AC5 比較メトリック ----
        "variant": variant,
        "dist": dist,
        "glue_loc": glue_loc,
        "dom_mutations": dom,
        "dom_mutations_per_meter_update": per_meter,
        // injected（FW 側 driver）か createMeterHost（命令的 DOM）かの証跡。
        "meters_driver": stats.get("meters_driver"),
        // ---- 同一ワークロードが走った証跡（render と同じ集計）----
        "backend": s.backend,
        "backend_used": stats.get("backend_used"),
        "fallback_to": stats.get("fallback"),
        "frames": stats.get("frames"),
        "frames_requested": s.frames,
        "elapsed_ms": stats.get("elapsed_ms"),
        "frame_time_ms": metrics::summarize(&frame_samples),
        "work_time_ms": metrics::summarize(&work_samples),
        "dropped": stats.get("dropped"),
        "achieved_fps": stats.get("achieved_fps"),
        // ---- ハウスキーピング ----
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

/// 終了評価（render と同じ規約: done.stats に 1 フレーム以上の記録で passed）。
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
