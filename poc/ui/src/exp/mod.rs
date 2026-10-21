//! 実験レジストリと `Experiment` trait。
//!
//! 実験は `setup`（前提ゲーティング）→ `run`（本体）→ `teardown`（後始末）の
//! ライフサイクルを持つ。`exp/*` は `host.rs` の内部 I/F（`HostEvent`/`Host`）
//! と `check::Environment` にのみ依存し、wry/tao/winreg 等の外部型は触れない
//! （design「外部型を境界の背後に置く」評価軸）。

pub mod host;

use serde_json::Value;

use crate::args::RunArgs;
use crate::check::Environment;
use crate::metrics::Status;

/// 前提不足によるスキップ（`status:"skipped"` + 非空 reason が契約）。
pub struct Skip(pub String);

/// 実験の実行結果。メトリック本体と終了ステータスの組。
pub struct Outcome {
    /// `poc-metric.v1` の `metrics` フィールド値。
    pub metrics: Value,
    /// 終了ステータス。
    pub status: Status,
}

/// 実験の共通ライフサイクル。
pub trait Experiment {
    /// `--experiment` で指定する名前。
    fn name(&self) -> &'static str;

    /// 前提確認。`Err(Skip)` は実行未実施（前提不足）を意味する。
    fn setup(&mut self, env: &Environment) -> Result<(), Skip>;

    /// 実験本体。必ずメトリックとステータスを返す
    /// （部分失敗は `Status::Failed` + 収集済みメトリックで表現する）。
    fn run(&mut self, env: &Environment, args: &RunArgs) -> Outcome;

    /// 後始末（リソース解放等。既定は何もしない）。
    fn teardown(&mut self) {}
}

/// 実験名 → 実験インスタンスのレジストリ。
/// T1 は `host`（AC1）のみ。T2-T5 が render/ipc/surface/frontend/dist を追加する。
pub fn lookup(name: &str) -> Option<Box<dyn Experiment>> {
    match name {
        "host" => Some(Box::new(host::HostExperiment::new())),
        _ => None,
    }
}

/// `--help` / エラーメッセージ用の既知実験名一覧。
pub fn known_names() -> &'static [&'static str] {
    &["host"]
}
