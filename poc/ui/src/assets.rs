//! 埋め込みアセットと `poc://` カスタムプロトコル応答。
//!
//! - `poc://app/...`  : `frontend/dist/` の埋め込み静的アセット配信
//!   （rust-embed: debug ビルドは FS 読み・release ビルドは埋め込み。
//!   dev server 不使用の証明 — AC1/AC6）。
//! - `poc://data/...` : 計測用バイナリ応答の名前空間（T3/AC4 が実装する。
//!   T1 では予約のみで 404 を返す）。
//!
//! Windows/Android では wry が `poc://app/x` を `http://poc.app/x` に書き換えて
//! ハンドラへ渡すため、host 名から `poc.` プレフィックスを剥がして名前空間を
//! 判定する。

use std::borrow::Cow;
use std::path::Path;

use http::{Request, Response, StatusCode};
use rust_embed::RustEmbed;

/// `frontend/dist/` を埋め込み対象とするアセット集合。
/// dist/ が無い環境では build.rs がスタブ index.html を生成するため、
/// 埋め込み自体は常に成立する（frontend 未ビルド判定は `frontend_built()`）。
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct AppAssets;

/// `poc://` プロトコルハンドラ（wry `with_custom_protocol` に登録する）。
/// 外部型（http::Request/Response）を含むため adapter 層の host.rs からのみ呼ぶ。
pub fn handle_poc_protocol(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let uri = request.uri();
    // Windows では `http://poc.app/...`、他 OS では `poc://app/...` の host 部分。
    let namespace = uri
        .host()
        .unwrap_or_default()
        .strip_prefix("poc.")
        .unwrap_or_else(|| uri.host().unwrap_or_default());
    match namespace {
        "app" => serve_app(uri.path()),
        "data" => serve_data(request),
        _ => not_found(format!("unknown poc namespace '{}'", uri)),
    }
}

/// フロントエンドが実際にビルドされているか（`.ui-poc-built` マーカーの有無）。
/// false の場合、埋め込み物は build.rs 生成のスタブであり、実験は
/// `skipped("frontend not built")` とすべき前提不足状態。
pub fn frontend_built() -> bool {
    AppAssets::get(".ui-poc-built").is_some()
}

/// `poc://app/` 配下の指定アセットが存在するか。
/// `frontend_built()` が通っても variant 別 dist（`vanilla/index.html` 等）が
/// 欠ける部分ビルド状態を実験側の前提検査で検出するために使う。
pub fn app_asset_exists(rel: &str) -> bool {
    AppAssets::get(rel).is_some()
}

/// `poc://app/...` の配信。`/` または空パスは `index.html` に解決する。
fn serve_app(path: &str) -> Response<Cow<'static, [u8]>> {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if !is_safe_path(rel) {
        return not_found(format!("invalid asset path '{rel}'"));
    }
    match AppAssets::get(rel) {
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_for(rel))
            .header("Access-Control-Allow-Origin", "*")
            .body(file.data)
            .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR)),
        None => not_found(format!("asset not found '{rel}'")),
    }
}

/// `poc://data/...`: 計測バイナリ応答の予約名前空間（T3/AC4 で実装）。
/// T1 では名前空間の分離のみを示し、未実装であることを明示して 404 を返す。
fn serve_data(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let _ = request;
    not_found("poc://data/ is reserved for AC4 binary-pull measurement (not implemented in T1)")
}

fn not_found(msg: impl AsRef<str>) -> Response<Cow<'static, [u8]>> {
    let body = msg.as_ref().as_bytes().to_vec();
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Owned(body))
        .unwrap_or_else(|_| empty_response(StatusCode::NOT_FOUND))
}

fn empty_response(status: StatusCode) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .body(Cow::Borrowed(&[][..]))
        .expect("empty response must build")
}

/// `..` や絶対パスによる埋め込み名空間外アクセスを拒否する。
/// （release 埋め込みでは dict 参照のみだが、debug の FS 読みで実害があるため防御）
fn is_safe_path(rel: &str) -> bool {
    !Path::new(rel).components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    })
}

/// 拡張子からの最小 Content-Type マッピング。
fn mime_for(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}
