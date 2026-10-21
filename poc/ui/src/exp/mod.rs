//! 実験レジストリと `Experiment` trait。
//!
//! 実験は `setup`（前提ゲーティング）→ `run`（本体）→ `teardown`（後始末）の
//! ライフサイクルを持つ。`exp/*` は `host.rs` の内部 I/F（`HostEvent`/`Host`）
//! と `check::Environment` にのみ依存し、wry/tao/winreg 等の外部型は触れない
//! （design「外部型を境界の背後に置く」評価軸）。

pub mod host;
pub mod ipc;
pub mod render;

use serde_json::Value;

use crate::args::RunArgs;
use crate::assets;
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

/// 全実験共通の前提ゲーティング（GUI・WebView2・frontend ビルド）。
/// 前提不足は `status:"skipped"`（test-spec のゲーティング契約）であり、
/// ゲーティング条件を実験ごとに重複させないためここに集約する。
/// 実験固有の追加前提（variant 別 dist・GPU capability 等）は各 setup で続けて検査する。
pub fn check_gui_prereqs(env: &Environment) -> Result<(), Skip> {
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

/// 実験名 → 実験インスタンスのレジストリ。
/// T1 は `host`（AC1）、T2 が `render`（AC2）、T3 が `ipc`（AC4）を追加。
/// T4/T5 が残りを追加する。
pub fn lookup(name: &str) -> Option<Box<dyn Experiment>> {
    match name {
        "host" => Some(Box::new(host::HostExperiment::new())),
        "render" => Some(Box::new(render::RenderExperiment::new())),
        "ipc" => Some(Box::new(ipc::IpcExperiment::new())),
        _ => None,
    }
}

/// `--help` / エラーメッセージ用の既知実験名一覧。
pub fn known_names() -> &'static [&'static str] {
    &["host", "render", "ipc"]
}
