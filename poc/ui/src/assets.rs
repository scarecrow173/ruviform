//! 埋め込みアセットと `poc://` カスタムプロトコル応答。
//!
//! - `poc://app/...`  : `frontend/dist/` の埋め込み静的アセット配信
//!   （rust-embed: debug ビルドは FS 読み・release ビルドは埋め込み。
//!   dev server 不使用の証明 — AC1/AC6）。
//! - `poc://data/...` : 計測用バイナリ応答の名前空間（AC4 バイナリ pull 経路。
//!   `poc://data/blob?bytes=N` が合成 `Cow` バッファを返す）。
//!
//! Windows/Android では wry が `poc://app/x` を `http://poc.app/x` に書き換えて
//! ハンドラへ渡すため、host 名から `poc.` プレフィックスを剥がして名前空間を
//! 判定する。

use std::borrow::Cow;
use std::path::Path;
use std::sync::OnceLock;

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

/// `poc://data/blob` が返せる最大バイト数（実験上限 16MiB に余裕を持たせた 32MiB）。
const MAX_BLOB_BYTES: usize = 32 * 1024 * 1024;

/// `poc://data/blob` 用の遅延初期化バッファ。初回要求時に 1 度だけ確保し、
/// 以後は先頭 `bytes` バイトを `Cow::Borrowed` で返す。要求ごとの確保/充填を
/// 避け、計測対象を「プロトコル転送そのもの」に絞るための設計。
static BLOB_BUF: OnceLock<Vec<u8>> = OnceLock::new();

/// 合成バッファ（内容は転送バイト数のみが計測対象のため決定論的パターン）。
fn blob_buf() -> &'static [u8] {
    BLOB_BUF.get_or_init(|| {
        let mut v = vec![0u8; MAX_BLOB_BYTES];
        for (i, b) in v.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(0x9E37_79B1) as u8;
        }
        v
    })
}

/// `poc://data/...`: 計測バイナリ応答（AC4 バイナリ pull 経路）。
/// JS 側の `fetch("poc://data/blob?bytes=N")` → `arrayBuffer()` までの経路を
/// 通すため、CORS ヘッダ（`poc://app` → `poc://data` はオリジン横断）を付ける。
fn serve_data(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    match request.uri().path().trim_start_matches('/') {
        "blob" => serve_blob(request.uri().query()),
        other => not_found(format!("unknown data path '{other}'")),
    }
}

/// `poc://data/blob?bytes=N` の合成バッファ応答。
/// `bytes` 欠落・非数値・範囲外は 400。`Access-Control-Allow-Origin: *` で
/// `poc://app` オリジンからの fetch を許可する（app 応答と同じ方針）。
fn serve_blob(query: Option<&str>) -> Response<Cow<'static, [u8]>> {
    let bytes = query
        .and_then(|q| q.split('&').find_map(|pair| pair.strip_prefix("bytes=")))
        .and_then(|v| v.parse::<usize>().ok());
    match bytes {
        Some(n) if (1..=MAX_BLOB_BYTES).contains(&n) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/octet-stream")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Borrowed(&blob_buf()[..n]))
            .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR)),
        Some(n) => bad_request(format!("bytes={n} out of range (1..={MAX_BLOB_BYTES})")),
        None => bad_request("poc://data/blob requires ?bytes=<1..=MAX_BLOB_BYTES>"),
    }
}

fn bad_request(msg: impl AsRef<str>) -> Response<Cow<'static, [u8]>> {
    let body = msg.as_ref().as_bytes().to_vec();
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Owned(body))
        .unwrap_or_else(|_| empty_response(StatusCode::BAD_REQUEST))
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
