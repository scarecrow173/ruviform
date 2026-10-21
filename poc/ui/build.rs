//! ui-poc ビルドスクリプト。
//!
//! 責務:
//! - `frontend/dist/` が存在しない場合にスタブ `index.html` を生成する。
//!   rust-embed の `#[folder]` はコンパイル時にディレクトリの存在を要求するため、
//!   フロントエンド未ビルド（`npm ci && npm run build` 未実施）でも
//!   `cargo build` / `--check` が常に動くようにする。実行側は埋め込みマーカー
//!   `.ui-poc-built` の有無で「frontend not built」による skipped 判定を行う。
//! - ビルド時の rustc バージョンを `UI_POC_RUSTC` 環境変数として焼き込む
//!   （`poc-metric.v1` の `env.rustc` に入れる値。実行環境に rustc は不要）。

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const STUB_HTML: &str = "<!doctype html><html><body><p>ui-poc: frontend not built. \
Run `npm ci && npm run build` in poc/ui/frontend.</p></body></html>\n";

fn main() {
    let dist = Path::new("frontend/dist");
    let marker = dist.join(".ui-poc-built");
    if !dist.join("index.html").exists() {
        fs::create_dir_all(dist).expect("create frontend/dist stub dir");
        fs::write(dist.join("index.html"), STUB_HTML).expect("write stub index.html");
        // スタブ生成時は必ず未ビルド扱いにするため、ビルドマーカーが残っていれば消す。
        if marker.exists() {
            fs::remove_file(&marker).expect("remove stale dist marker");
        }
    }
    println!("cargo:rerun-if-changed=frontend/dist");

    // ビルドに使った rustc のバージョン文字列を取得して焼き込む。
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=UI_POC_RUSTC={version}");
}
