//! `GoldenKey` / `EnvironmentMeta` — baseline 同一性と capture 環境の
//! 証拠 DTO（Issue #314 slice 1）。
//!
//! `GoldenKey` は baseline を一意に指す**安定した同一性**であり、
//! Story ID + Theme + State + Driver/OS/GPU class + DPI/font +
//! Revision で構成する（Issue #314 確定契約）。`EnvironmentMeta` は
//! capture 実行時に観測した環境の**証拠**であり、key の env 側
//! フィールドはここから正規化された class 文字列を受け取る —
//! 生値の丸め規則は capture 層（slice 3）の責務で、本 module は
//! 値を捏造しない（取得不能は `None`/`unknown`）。
//!
//! 全て driver-neutral な DTO であり、GPUI/wry/OS handle 型を含まない。
//!
//! **RT contract:** non-RT only.

use serde::{Deserialize, Serialize};

/// baseline を一意に指す golden key。
///
/// `story_id`/`theme`/`state` は scenario 同一性（driver-lab
/// `stories.json` の story ID と同じ名前空間を想定するが、本 slice
/// では registry への結合・実在検査を行わない）。`driver_class`/
/// `os_class`/`gpu_class`/`scale_factor`/`font_class` は環境 class、
/// `revision` は ruviform/アプリ側の revision。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenKey {
    /// Story ID（例: `LAB-SV-FADER-DOCK`）。
    pub story_id: String,
    /// Theme 名（例: `synvelis-dark`）。
    pub theme: String,
    /// Semantic state 名（例: `idle`/`focus`）。
    pub state: String,
    /// Capture driver/backend の class（例: `gpui-offscreen`）。
    pub driver_class: String,
    /// OS class（例: `windows`/`macos`/`linux`）。
    pub os_class: String,
    /// GPU class。環境を区別できない場合は `None` — 生 GPU 名ではなく
    /// 正規化 class を入れる（生値は `EnvironmentMeta::gpu` が保持）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_class: Option<String>,
    /// Scale factor（DPI 正規化値。例: `1.0`/`2.0`）。
    pub scale_factor: f64,
    /// Font 環境 class。font raster に影響する環境差を分離するための
    /// 正規化識別子。取得不能は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_class: Option<String>,
    /// ruviform/アプリ revision（baseline がどの実装に対するものか）。
    pub revision: String,
}

impl GoldenKey {
    /// evidence 用の安定 canonical 文字列。全要素を `/` で連結し、
    /// `None` の class は `unknown` と表記する（捏造しない）。
    #[must_use]
    pub fn canonical(&self) -> String {
        format!(
            "{}/{}/{}/{}/{}/{}/sf{}/{}/r{}",
            self.story_id,
            self.theme,
            self.state,
            self.driver_class,
            self.os_class,
            self.gpu_class.as_deref().unwrap_or("unknown"),
            fmt_scale(self.scale_factor),
            self.font_class.as_deref().unwrap_or("unknown"),
            self.revision,
        )
    }

    /// file system 安全な baseline ファイル stem
    /// （`<driver>__<os>__<gpu>__sf<scale>__<font>__r<rev>`）。
    /// `[A-Za-z0-9._-]` 以外は `_` に置換する。
    #[must_use]
    pub fn file_stem(&self) -> String {
        let gpu = self.gpu_class.as_deref().unwrap_or("unknown");
        let font = self.font_class.as_deref().unwrap_or("unknown");
        sanitize(&format!(
            "{}__{}__{}__sf{}__{}__r{}",
            self.driver_class,
            self.os_class,
            gpu,
            fmt_scale(self.scale_factor),
            font,
            self.revision,
        ))
    }

    /// baseline store の directory segment（`story/theme/state`）。
    /// path 安全のため各要素を sanitize する。
    #[must_use]
    pub fn path_segments(&self) -> [String; 3] {
        [
            sanitize(&self.story_id),
            sanitize(&self.theme),
            sanitize(&self.state),
        ]
    }
}

/// `f64` scale factor の決定的な文字列化（`2.0` → `"2.0"`）。
fn fmt_scale(v: f64) -> String {
    // `{:?}` は f64 を最短往復表現で出す（2.0 → "2.0"、1.5 → "1.5"）。
    format!("{v:?}")
}

/// `[A-Za-z0-9._-]` 以外の文字を `_` に置換する path/file stem 用
/// sanitize。空文字列は `"_"` にする（segment が消えないように）。
fn sanitize(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() { "_".to_string() } else { out }
}

/// GPU に関する観測値。class への正規化は capture 層の責務で、
/// ここでは観測できた生の識別情報のみを保持する。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuMeta {
    /// GPU アダプタ名（例: `NVIDIA GeForce RTX 4090`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// GPU vendor（例: `nvidia`/`amd`/`intel`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    /// 正規化 GPU class（baseline key の `gpu_class` に対応）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

/// pixel の取得経路。Issue #314 は offscreen 実測を優先し、非対応
/// 環境は実 Window capture 等の実証済み経路へ委譲する — どの経路で
/// 得た pixel かを証拠として区別する。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePath {
    /// Offscreen render target からの readback。
    Offscreen,
    /// 実 Window の capture（OS 画面キャプチャ等）。
    RealWindow,
    /// 本 utility 外の経路で取得した pixel（例: OS 標準
    /// スクリーンショットツール）。
    External,
    /// 取得経路が記録されていない/不明 — 成功扱いしないための
    /// 正直な既定値。
    Unknown,
}

impl CapturePath {
    /// Wire 名。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Offscreen => "offscreen",
            Self::RealWindow => "real_window",
            Self::External => "external",
            Self::Unknown => "unknown",
        }
    }
}

/// capture 実行時の環境証拠（evidence / baseline sidecar に記録）。
///
/// `os`/`arch` は `std::env::consts` 相当の class 文字列を想定するが、
/// 型としては単なる `String` — capture 側が実測値を入れる。
/// 取得不能な項目は `None` のまま記録し、捏造しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentMeta {
    /// OS class（例: `windows`/`macos`/`linux`）。
    pub os: String,
    /// Arch class（例: `x86_64`/`aarch64`）。
    pub arch: String,
    /// OS version/build。取得不能は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    /// Capture backend/driver 名（例: `gpui-test`, `wgc`）。
    pub backend: String,
    /// Backend の版・詳細。取得不能は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_detail: Option<String>,
    /// GPU 観測値。GPU を持たない/不明な環境は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuMeta>,
    /// Scale factor（DPI 正規化値）。
    pub scale_factor: f64,
    /// Font 環境の識別子（font 列・version 等の正規化表現）。
    /// 取得不能は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_set: Option<String>,
    /// pixel の取得経路。
    pub capture: CapturePath,
    /// ruviform crate version（`env!("CARGO_PKG_VERSION")` 相当を
    /// capture 側が入れる）。
    pub ruviform_revision: String,
    /// 本 utility/ツール側の revision。取得不能は `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> GoldenKey {
        GoldenKey {
            story_id: "LAB-SV-FADER-DOCK".to_string(),
            theme: "synvelis-dark".to_string(),
            state: "idle".to_string(),
            driver_class: "gpui-offscreen".to_string(),
            os_class: "windows".to_string(),
            gpu_class: Some("dx11".to_string()),
            scale_factor: 2.0,
            font_class: Some("inter-v13".to_string()),
            revision: "abc1234".to_string(),
        }
    }

    #[test]
    fn golden_key_canonical_covers_all_fields() {
        let base = key();
        let canonical = base.canonical();
        assert!(canonical.contains("LAB-SV-FADER-DOCK"));
        assert!(canonical.contains("synvelis-dark"));
        assert!(canonical.contains("gpui-offscreen"));
        assert!(canonical.contains("sf2.0"));
        // 全要素を 1 つずつ変えると canonical が変わる。
        let mut variants = Vec::new();
        let mut v = base.clone();
        v.story_id = "OTHER".to_string();
        variants.push(v);
        let mut v = base.clone();
        v.theme = "audiolens-dark".to_string();
        variants.push(v);
        let mut v = base.clone();
        v.state = "focus".to_string();
        variants.push(v);
        let mut v = base.clone();
        v.driver_class = "real-window".to_string();
        variants.push(v);
        let mut v = base.clone();
        v.os_class = "linux".to_string();
        variants.push(v);
        let mut v = base.clone();
        v.gpu_class = None;
        variants.push(v);
        let mut v = base.clone();
        v.scale_factor = 1.0;
        variants.push(v);
        let mut v = base.clone();
        v.font_class = None;
        variants.push(v);
        let mut v = base.clone();
        v.revision = "def5678".to_string();
        variants.push(v);
        for v in variants {
            assert_ne!(v.canonical(), canonical, "variant must differ: {v:?}");
        }
        // file_stem は env class + revision のみを畳み込む
        // （story/theme/state は path segment 側）。
        for (v, expect_diff) in [
            (base.clone(), false),
            ({
                let mut v = base.clone();
                v.story_id = "OTHER".to_string();
                v
            }, false),
            ({
                let mut v = base.clone();
                v.theme = "other".to_string();
                v
            }, false),
            ({
                let mut v = base.clone();
                v.driver_class = "real-window".to_string();
                v
            }, true),
            ({
                let mut v = base.clone();
                v.gpu_class = None;
                v
            }, true),
            ({
                let mut v = base.clone();
                v.scale_factor = 1.0;
                v
            }, true),
            ({
                let mut v = base.clone();
                v.font_class = None;
                v
            }, true),
            ({
                let mut v = base.clone();
                v.revision = "def5678".to_string();
                v
            }, true),
        ] {
            if expect_diff {
                assert_ne!(v.file_stem(), base.file_stem(), "{v:?}");
            } else {
                assert_eq!(v.file_stem(), base.file_stem(), "{v:?}");
            }
        }
    }

    #[test]
    fn golden_key_file_stem_is_fs_safe_and_deterministic() {
        let mut k = key();
        k.driver_class = "gpui offscreen/v2".to_string();
        k.os_class = "windows".to_string();
        k.revision = "rev/with space".to_string();
        let stem = k.file_stem();
        assert!(
            stem
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')),
            "stem must be fs-safe: {stem}"
        );
        assert_eq!(stem, k.file_stem(), "stem must be deterministic");
        assert!(stem.contains("sf2.0"));
    }

    #[test]
    fn golden_key_path_segments_are_sanitized() {
        let mut k = key();
        k.story_id = "LAB/../evil".to_string();
        k.theme = "theme name".to_string();
        let segs = k.path_segments();
        assert_eq!(segs.len(), 3);
        for s in &segs {
            assert!(!s.contains('/') && !s.contains('\\'), "segment: {s}");
        }
    }

    #[test]
    fn golden_key_serde_round_trip() {
        let k = key();
        let json = serde_json::to_string(&k).unwrap();
        let back: GoldenKey = serde_json::from_str(&json).unwrap();
        assert_eq!(k, back);
        // Option None は serialize で省略される。
        let mut k2 = key();
        k2.gpu_class = None;
        k2.font_class = None;
        let json2 = serde_json::to_string(&k2).unwrap();
        assert!(!json2.contains("gpu_class"));
        assert!(!json2.contains("font_class"));
        let back2: GoldenKey = serde_json::from_str(&json2).unwrap();
        assert_eq!(k2, back2);
    }

    #[test]
    fn environment_meta_serde_round_trip_keeps_none() {
        let env = EnvironmentMeta {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            os_version: None,
            backend: "test".to_string(),
            backend_detail: None,
            gpu: Some(GpuMeta {
                name: Some("Test GPU".to_string()),
                vendor: None,
                class: Some("test-class".to_string()),
            }),
            scale_factor: 1.5,
            font_set: None,
            capture: CapturePath::Offscreen,
            ruviform_revision: "0.1.0".to_string(),
            tool_revision: None,
        };
        let json = serde_json::to_string(&env).unwrap();
        let back: EnvironmentMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(env, back);
        assert!(!json.contains("os_version"));
        assert!(json.contains("\"capture\":\"offscreen\""));
    }

    #[test]
    fn capture_path_wire_names() {
        assert_eq!(CapturePath::Offscreen.as_str(), "offscreen");
        assert_eq!(CapturePath::RealWindow.as_str(), "real_window");
        assert_eq!(CapturePath::External.as_str(), "external");
        assert_eq!(CapturePath::Unknown.as_str(), "unknown");
    }
}
