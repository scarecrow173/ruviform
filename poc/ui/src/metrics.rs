//! `poc-metric.v1` 計測契約の実装（wry 型に触れない内部レイヤ）。
//!
//! - 全実験は終了時に 1 行の JSON を **stdout 最終行**に出力し、
//!   `--out`（既定 `out/<experiment>-<epoch>.jsonl`）へも追記する。
//! - スキーマ: `{schema, experiment, ts, env:{os,arch,webview2_version,
//!   wgpu_adapter,rustc}, params, metrics, status, reason?}`
//! - `status`/`reason` の規約（test-spec 正本）:
//!   `passed` → exit 0 / `skipped` + 非空 reason → exit 0 /
//!   `failed` → exit 1 / CLI・内部エラー → exit 2。
//! - 診断メッセージはすべて stderr に出し、stdout は最終 JSON 行のために予約する。

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::check::Environment;

/// 計測スキーマ識別子。
pub const SCHEMA: &str = "poc-metric.v1";

/// 実験の終了ステータス（exit code と独立した構造化判定）。
#[derive(Debug, Clone)]
pub enum Status {
    /// 全記録項目が成立。
    Passed,
    /// 前提不足（WebView2 不在・headless・frontend 未ビルド等）。reason 非空が契約。
    Skipped(String),
    /// 実験自体の失敗（タイムアウト・観測不能・検証不一致等）。
    Failed(String),
}

impl Status {
    /// `poc-metric.v1` の status 文字列。
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Passed => "passed",
            Status::Skipped(_) => "skipped",
            Status::Failed(_) => "failed",
        }
    }

    /// `reason` フィールド値（passed では None）。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Status::Passed => None,
            Status::Skipped(r) | Status::Failed(r) => Some(r.as_str()),
        }
    }

    /// exit code へのマッピング（0 = passed/skipped、1 = failed）。
    pub fn exit_code(&self) -> i32 {
        match self {
            Status::Passed | Status::Skipped(_) => 0,
            Status::Failed(_) => 1,
        }
    }
}

/// `poc-metric.v1` 1 レコード。
pub struct MetricRecord {
    /// 実験名（`--experiment` 値）。
    pub experiment: String,
    /// `check::Environment` が検出した env スナップショット。
    pub env: Value,
    /// `--frames`/`--duration`/`--backend` 等の実行パラメータ。
    pub params: Value,
    /// 実験別メトリック（例: protocol_load, ipc_echo, ...）。
    pub metrics: Value,
    /// 終了ステータス。
    pub status: Status,
}

impl MetricRecord {
    /// レコードを JSON 値へ変換する（`reason` は passed 時に省略）。
    pub fn to_json(&self) -> Value {
        let mut obj = Map::new();
        obj.insert("schema".into(), json!(SCHEMA));
        obj.insert("experiment".into(), json!(self.experiment));
        obj.insert("ts".into(), json!(epoch_secs()));
        obj.insert("env".into(), self.env.clone());
        obj.insert("params".into(), self.params.clone());
        obj.insert("metrics".into(), self.metrics.clone());
        obj.insert("status".into(), json!(self.status.as_str()));
        if let Some(reason) = self.status.reason() {
            obj.insert("reason".into(), json!(reason));
        }
        Value::Object(obj)
    }
}

/// env 付きレコード生成のヘルパ。
pub fn record(
    experiment: &str,
    env: &Environment,
    params: Value,
    metrics: Value,
    status: Status,
) -> MetricRecord {
    MetricRecord {
        experiment: experiment.to_string(),
        env: env.to_metric_env(),
        params,
        metrics,
        status,
    }
}

/// `--out` 未指定時の既定パス: `poc/ui/out/<experiment>-<epoch>.jsonl`。
/// `CARGO_MANIFEST_DIR` 基準にして、リポジトリルートから
/// `--manifest-path` 実行した場合でも証跡が PoC 側に置かれるようにする。
pub fn default_out_path(experiment: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("out")
        .join(format!("{experiment}-{}.jsonl", epoch_secs()))
}

/// JSONL 追記 + stdout 最終行出力。`out` はレコードごとに open-append する。
/// 失敗時は Err(io::Error)（呼び出し側は内部エラー exit 2 に対応付ける）。
pub fn emit(record: &MetricRecord, out: &Path) -> io::Result<()> {
    let line = serde_json::to_string(&record.to_json())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(out)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;

    // 最終 stdout 行として JSON を出す（契約）。
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(line.as_bytes())?;
    lock.write_all(b"\n")?;
    lock.flush()
}

/// UNIX epoch 秒（タイムスタンプ・ファイル名共通）。
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// サンプル列の集計値（avg/p50/p95/p99）。frame_time_ms・rtt_ms 等の
/// パーセンタイル集計に使う共通ルール（T2 以降の render/ipc 実験が利用する
/// 計測契約の一部）。
/// 空入力は全値 0.0 を返す（呼び出し側でサンプル数も記録すること）。
pub fn summarize(samples: &[f64]) -> Value {
    if samples.is_empty() {
        return json!({"count": 0, "avg": 0.0, "p50": 0.0, "p95": 0.0, "p99": 0.0});
    }
    let mut sorted: Vec<f64> = samples.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = sorted.len();
    if count == 0 {
        return json!({"count": 0, "avg": 0.0, "p50": 0.0, "p95": 0.0, "p99": 0.0});
    }
    let sum: f64 = sorted.iter().sum();
    json!({
        "count": count,
        "avg": sum / count as f64,
        "p50": percentile(&sorted, 50.0),
        "p95": percentile(&sorted, 95.0),
        "p99": percentile(&sorted, 99.0),
    })
}

/// 最近接順位法（nearest-rank）のパーセンタイル。`sorted` は昇順前提。
fn percentile(sorted: &[f64], p: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let rank = (p / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted[rank.max(1).min(sorted.len()) - 1]
}
