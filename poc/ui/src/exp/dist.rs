//! `--experiment dist`（AC6: 配布・単一バイナリ検証）。
//!
//! 検証項目（design/test-spec 正本）— GUI を作らず headless で動く実験:
//! - `webview2_version`（check.rs の winreg 検出の実機確認。
//!   HKLM/HKCU `...\Clients\{F3017226-...}` の `pv` 値）と検出可否。
//! - `binary_bytes`（`std::env::current_exe()` のサイズ）とプロファイル。
//! - `embedded_ok`: rust-embed が `poc://app/index.html` を供給できるか。
//!   release ビルドは埋め込み辞書、debug ビルドは FS 読みのため、
//!   `embed_mode` でどちらが応答したかを明示する
//!   （単体バイナリを temp dir へコピーして release で `embedded_ok:true`
//!   なら FS 非依存の証明になる）。
//! - `bootstrapper_guidance`: WebView2 未検出時に案内すべき
//!   Evergreen bootstrapper 手順のテキスト（検出済み環境でも文面を記録し、
//!   未検出時は stderr にも出力する）。
//!
//! `check_gui_prereqs` は呼ばない（headless 契約）。記録項目が取れれば
//! `passed`、前提不足の skip は設けない。

use serde_json::{Value, json};

use crate::args::RunArgs;
use crate::assets;
use crate::check::{self, Environment};
use crate::metrics::Status;

use super::{Experiment, Outcome, Skip};

/// AC6 実験（非 GUI。状態を持たない）。
pub struct DistExperiment;

/// WebView2 未検出時の bootstrapper 案内文面（メトリック記録と stderr 出力の
/// 共用正本）。Microsoft 公式の Evergreen bootstrapper リンクと winget ID。
const BOOTSTRAPPER_GUIDANCE: &str = "WebView2 Evergreen Runtime not detected. \
Install via the official bootstrapper \
(https://go.microsoft.com/fwlink/p/?LinkId=2124703) \
or `winget install Microsoft.EdgeWebView2Runtime`, then re-run this experiment.";

impl DistExperiment {
    /// 実験インスタンス生成（レジストリから呼ばれる）。
    pub fn new() -> Self {
        Self
    }
}

/// rust-embed の供給経路（release=埋め込み / debug=FS 読み）。
/// バイナリ単体コピー検証で「FS が無くても応答するか」の判定根拠。
const fn embed_mode() -> &'static str {
    if cfg!(debug_assertions) {
        "fs-debug (rust-embed reads frontend/dist from filesystem)"
    } else {
        "embedded-release (rust-embed embeds frontend/dist into the binary)"
    }
}

impl Experiment for DistExperiment {
    fn name(&self) -> &'static str {
        "dist"
    }

    fn setup(&mut self, _env: &Environment) -> Result<(), Skip> {
        // GUI 前提を一切要求しない（check_gui_prereqs を呼ばない設計）。
        // headless / 未ビルド / WebView2 未検出は全て「記録される値」であり、
        // 前提不足による skip は存在しない。
        Ok(())
    }

    fn run(&mut self, env: &Environment, _args: &RunArgs) -> Outcome {
        let webview2_detected = env.webview2_version.is_some();
        if !webview2_detected {
            // 未検出環境では案内を stderr に出す（stdout 最終行は metrics のため予約）。
            eprintln!("{BOOTSTRAPPER_GUIDANCE}");
        }

        // 埋め込みアセット応答の検証。
        let index_html = assets::app_asset_data("index.html");
        let embedded_ok = index_html
            .as_ref()
            .is_some_and(|d| d.windows(5).any(|w| w == b"<html" || w == b"<!doc"));

        let metrics = json!({
            // ---- AC6 必須メトリック ----
            "webview2_version": env.webview2_version,
            "webview2_detected": webview2_detected,
            "binary_bytes": binary_bytes(),
            "embedded_ok": embedded_ok,
            "bootstrapper_guidance": BOOTSTRAPPER_GUIDANCE,
            "bootstrapper_required": !webview2_detected,
            // ---- 補助証跡 ----
            "embed_mode": embed_mode(),
            "frontend_built": assets::frontend_built(),
            "app_asset_count": assets::app_asset_names().len(),
            "index_html_bytes": index_html.as_ref().map(Vec::len),
            "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            // --check と同一の検出経路のスナップショット（比較用）。
            "check": check::check_json(env),
        });

        Outcome {
            metrics,
            status: Status::Passed,
        }
    }
}

/// 実行中バイナリ自身のサイズ（バイト）。取得失敗は metrics 内で null に
/// 潰さず、current_exe 自体の失敗のみ `Err` を返す（実験失敗扱いにせず
/// 観測値として記録する方針 — None は「サイズ不明」の構造化値）。
fn binary_bytes() -> Value {
    match std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
    {
        Some(m) => json!(m.len()),
        None => Value::Null,
    }
}
