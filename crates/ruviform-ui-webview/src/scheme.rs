//! `rvf://` custom-protocol response handler — the single
//! implementation of asset serving, bounded data pulls, MIME mapping,
//! traversal rejection and CORS for the app origin.
//!
//! Namespaces:
//!
//! - `rvf://app/<path>` → [`AssetSource`] (`index.html` default,
//!   extension MIME, path safety via `ruviform_ui::resolve_path`).
//! - `rvf://data/<name>?offset=N&len=M` → [`DataRegistry`] bounded
//!   read (`len` capped at `DataRegistry::max_chunk`,
//!   `X-Rvf-Total` echoes the full length for paging).
//!
//! Windows note: `WebView2` rewrites custom schemes to
//! `http://rvf.app/...` — the handler strips an `rvf.` host prefix so
//! both forms resolve identically (the `poc/ui` behavior, inherited).
//!
//! Kept free of windowing types so every path is headless-testable.

use std::borrow::Cow;
use std::sync::Arc;

use http::{Request, Response};

use ruviform_ui::{AssetSource, DataRegistry, DataRequest, mime_for, resolve_path};

type CowBody = Cow<'static, [u8]>;

/// Serves one `rvf://` request.
pub(crate) fn serve(
    assets: &Arc<dyn AssetSource>,
    data: &DataRegistry,
    request: &Request<Vec<u8>>,
) -> Response<CowBody> {
    let uri = request.uri();
    let host = uri.host().unwrap_or_default();
    // Windows rewrites rvf://<ns>/<path> to http://rvf.<ns>/<path>.
    let namespace = host.strip_prefix("rvf.").unwrap_or(host);
    match namespace {
        "app" => serve_app(assets.as_ref(), uri.path()),
        "data" => serve_data(data, uri.path(), uri.query()),
        _ => plain(404, "unknown rvf namespace"),
    }
}

fn serve_app(assets: &dyn AssetSource, raw_path: &str) -> Response<CowBody> {
    let Some(path) = resolve_path(raw_path) else {
        return plain(400, "rejected asset path");
    };
    match assets.get(&path) {
        Some(asset) => Response::builder()
            .status(200)
            .header("Content-Type", asset.mime)
            .header("Access-Control-Allow-Origin", "*")
            .body(asset.bytes)
            .unwrap_or_else(|_| plain(500, "response build failed")),
        None => plain(404, "asset not found"),
    }
}

fn serve_data(data: &DataRegistry, raw_name: &str, query: Option<&str>) -> Response<CowBody> {
    // Names are URI-safe slugs minted by Rust; multi-segment names
    // (`demo/peaks`) are allowed — every segment must be non-empty and
    // not `..`.
    let name = raw_name.trim_start_matches('/');
    if name.is_empty() || name.split('/').any(|s| s.is_empty() || s == "..") {
        return plain(400, "rejected data source name");
    }
    let (offset, len) = match parse_query(query) {
        Ok(v) => v,
        Err(msg) => return plain(400, msg),
    };
    match data.read(&DataRequest {
        name: name.to_string(),
        offset,
        len,
    }) {
        Ok(chunk) => Response::builder()
            .status(200)
            .header("Content-Type", mime_for("blob.bin"))
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .header("X-Rvf-Total", chunk.total.to_string())
            .body(Cow::Owned(chunk.bytes))
            .unwrap_or_else(|_| plain(500, "response build failed")),
        Err(e) => plain(404, &e.to_string()),
    }
}

/// Parses `offset=N&len=M` (both optional; `offset` defaults to 0).
fn parse_query(query: Option<&str>) -> Result<(u64, Option<u64>), &'static str> {
    let mut offset = 0u64;
    let mut len = None;
    let Some(query) = query else {
        return Ok((offset, len));
    };
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once('=') else {
            return Err("malformed query pair");
        };
        match key {
            "offset" => {
                offset = value
                    .parse::<u64>()
                    .map_err(|_| "offset must be a non-negative integer")?;
            }
            "len" => {
                len = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| "len must be a non-negative integer")?,
                );
            }
            _ => {}
        }
    }
    Ok((offset, len))
}

fn plain(status: u16, message: &str) -> Response<CowBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Owned(message.as_bytes().to_vec()))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b"error"[..])))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::Request;

    use ruviform_ui::{DataRegistry, MapAssetSource, VecDataSource};

    use super::serve;

    fn fixture() -> (Arc<dyn ruviform_ui::AssetSource>, DataRegistry) {
        let assets: Arc<dyn ruviform_ui::AssetSource> = Arc::new(
            MapAssetSource::new()
                .with_asset("index.html", None, "<html/>".as_bytes())
                .with_asset("app.js", None, "let a=1;".as_bytes()),
        );
        let data = DataRegistry::with_max_chunk(8);
        data.register("demo/peaks", Arc::new(VecDataSource::new(vec![1u8; 64])));
        (assets, data)
    }

    fn get(url: &str) -> Request<Vec<u8>> {
        Request::builder().uri(url).body(Vec::new()).unwrap()
    }

    #[test]
    fn app_serves_index_and_mime() {
        let (a, d) = fixture();
        let r = serve(&a, &d, &get("rvf://app/index.html"));
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers()["Content-Type"], "text/html; charset=utf-8");
        assert_eq!(r.body().as_ref(), b"<html/>");
        let r = serve(&a, &d, &get("rvf://app/app.js"));
        assert_eq!(
            r.headers()["Content-Type"],
            "text/javascript; charset=utf-8"
        );
        let r = serve(&a, &d, &get("rvf://app/"));
        assert_eq!(r.status(), 200, "empty path resolves to index.html");
    }

    #[test]
    fn app_rejects_traversal_and_missing() {
        let (a, d) = fixture();
        for path in [
            "rvf://app/../secret",
            "rvf://app/a/../../b",
            "rvf://app/c:\\windows",
            "rvf://app/a//b",
        ] {
            let r = serve(&a, &d, &get(path));
            assert!(r.status().is_client_error(), "{path} must not resolve");
        }
        assert_eq!(serve(&a, &d, &get("rvf://app/missing.js")).status(), 404);
    }

    #[test]
    fn data_serves_bounded_ranges() {
        let (a, d) = fixture();
        // Names are raw path segments — %2F is not decoded.
        let r = serve(&a, &d, &get("rvf://data/demo%2Fpeaks?offset=0&len=4"));
        assert_eq!(r.status(), 404);

        let (a, d) = fixture();
        let r = serve(&a, &d, &get("rvf://data/demo/peaks?offset=10&len=4"));
        assert_eq!(r.status(), 200);
        assert_eq!(r.body().len(), 4);
        assert_eq!(r.headers()["X-Rvf-Total"], "64");

        // len capped at max_chunk (8)
        let r = serve(&a, &d, &get("rvf://data/demo/peaks?offset=0"));
        assert_eq!(r.body().len(), 8);
        let r = serve(&a, &d, &get("rvf://data/demo/peaks?offset=60&len=100"));
        assert_eq!(r.body().len(), 4);
    }

    #[test]
    fn data_rejects_bad_requests() {
        let (a, d) = fixture();
        for url in [
            "rvf://data/demo/peaks?offset=abc",
            "rvf://data/demo/peaks?offset=-1",
            "rvf://data/demo/peaks?offset=65", // beyond end
            "rvf://data/missing?offset=0",
            "rvf://data/a//b?offset=0",
            "rvf://data/../peaks?offset=0",
        ] {
            let r = serve(&a, &d, &get(url));
            assert!(r.status().is_client_error(), "{url} must fail");
        }
    }

    #[test]
    fn windows_rewritten_host_resolves() {
        let (a, d) = fixture();
        let r = serve(&a, &d, &get("http://rvf.app/index.html"));
        assert_eq!(r.status(), 200);
        let r = serve(&a, &d, &get("http://rvf.data/demo/peaks?offset=0&len=2"));
        assert_eq!(r.status(), 200);
        assert_eq!(r.body().len(), 2);
    }

    #[test]
    fn cors_header_present() {
        let (a, d) = fixture();
        let r = serve(&a, &d, &get("rvf://data/demo/peaks?offset=0&len=1"));
        assert_eq!(r.headers()["Access-Control-Allow-Origin"], "*");
    }
}
