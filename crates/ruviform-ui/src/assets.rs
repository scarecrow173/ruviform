//! Frontend asset delivery port.
//!
//! `rvf://app/<path>` resolves through an [`AssetSource`]. The
//! foundation ships [`MapAssetSource`] for embedded/static assets and
//! `ruviform-ui-webview` provides a `rust-embed` adapter for the
//! production single-binary layout (debug: filesystem passthrough,
//! release: embedded — the `poc/ui` model).
//!
//! Path handling is a security boundary: [`resolve_path`] is the single
//! implementation of the traversal/absolute-path policy used by the
//! scheme handler.

use std::borrow::Cow;
use std::collections::BTreeMap;

/// A frontend asset (path-served bytes plus MIME type).
#[derive(Debug, Clone)]
pub struct Asset {
    /// Asset bytes (`Cow` lets embedded sources lend static data).
    pub bytes: Cow<'static, [u8]>,
    /// MIME type for the `Content-Type` header.
    pub mime: &'static str,
}

/// Port for resolving `rvf://app` paths to assets.
///
/// **RT contract:** non-RT only — served on the scheme handler thread.
pub trait AssetSource: Send + Sync {
    /// Looks up `path` (already normalized by [`resolve_path`]).
    fn get(&self, path: &str) -> Option<Asset>;
}

/// In-memory [`AssetSource`] — the minimal implementation used by the
/// demo app, tests, and small tools that hand-embed a handful of files.
pub struct MapAssetSource {
    map: BTreeMap<String, Asset>,
}

impl Default for MapAssetSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MapAssetSource {
    /// Empty map source.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Builder-style asset insertion (`mime` defaults via
    /// [`mime_for`] when `None`).
    #[must_use]
    pub fn with_asset(
        mut self,
        path: &str,
        mime: Option<&'static str>,
        bytes: impl Into<Cow<'static, [u8]>>,
    ) -> Self {
        self.insert(path, mime, bytes);
        self
    }

    /// Inserts an asset at `path` (leading `/` tolerated).
    pub fn insert(
        &mut self,
        path: &str,
        mime: Option<&'static str>,
        bytes: impl Into<Cow<'static, [u8]>>,
    ) {
        let path = path.trim_start_matches('/').to_string();
        let mime = mime.unwrap_or_else(|| mime_for(&path));
        self.map.insert(
            path,
            Asset {
                bytes: bytes.into(),
                mime,
            },
        );
    }
}

impl AssetSource for MapAssetSource {
    fn get(&self, path: &str) -> Option<Asset> {
        self.map.get(path).cloned()
    }
}

/// Normalizes a request path from `rvf://app/<path>` into a lookup key.
///
/// Rules (the traversal policy, single implementation):
/// - empty path, `/` or `index` variants resolve to `index.html`;
/// - leading `/` is stripped;
/// - `..` segments, backslashes, `:` (Windows drive/ADS), control
///   characters, and consecutive `//` are rejected (`None`).
///
/// Percent-decoding is **not** performed — asset names must be URI
/// paths already; the scheme handler serves raw UTF-8 paths.
#[must_use]
pub fn resolve_path(raw: &str) -> Option<String> {
    let path = raw.strip_prefix('/').unwrap_or(raw);
    if path.is_empty() {
        return Some("index.html".to_string());
    }
    if path.starts_with('/') {
        return None; // "//host/path" is a protocol-relative form, not an asset
    }
    if path
        .chars()
        .any(|c| c == '\\' || c == ':' || c.is_control())
    {
        return None;
    }
    let mut parts = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | ".." => return None, // consecutive "//", trailing "/", traversal
            "." => {}
            s => parts.push(s),
        }
    }
    if parts.is_empty() {
        return Some("index.html".to_string());
    }
    Some(parts.join("/"))
}

/// MIME type for a normalized asset path (extension-based).
#[must_use]
pub fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
