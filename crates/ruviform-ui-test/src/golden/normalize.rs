//! `GoldenKey` の環境正規化と fixture registry 結合
//! （Issue #314 slice 5・設計 `docs/designs/issue-314-golden-evidence.md`）。
//!
//! `EnvironmentMeta`（capture 時に観測した環境証拠）から
//! `GoldenKey` の env 側 class を導出する規則をここに集約する。
//! slice 1 の `key.rs` は「生値の丸め規則は capture 層の責務」
//! としたが、capture 層は backend のみを提供し key 構築規則を
//! 持たなかった — 本 module がその正本であり、consumer は
//! [`key_env`] → [`key_for_fixture`] の順で `GoldenKey` を得る。
//!
//! - [`key_env`]: `EnvironmentMeta` + caller 宣言の driver class
//!   → [`GoldenKeyEnv`]（os/gpu/scale/font の正規化）。
//! - [`key_for_fixture`]: [`StoryFixture`]（#426 code registry）
//!   に結合した `GoldenKey` 構築 — `story_id`/`theme` は fixture
//!   から写すため registry drift が構造的に起きない。
//! - [`registry_mismatches`]: `StoryManifest`（#390 `stories.json`
//!   JSON registry）に対する `story_id`/`theme`/`state`/
//!   `scale_factor` の整合検査 — 強制ではなく監査手段。
//!
//! 正規化は決定的で推測しない: 取得不能な値は `None` のまま、
//! 不正な scale factor は [`NormalizeError`] で拒否する。
//!
//! **RT contract:** non-RT only.

use std::fmt;

use crate::driver_lab::StoryManifest;
use crate::golden::key::{EnvironmentMeta, GoldenKey};
use crate::lab_story::StoryFixture;

/// `GoldenKey` の env 側 class — `EnvironmentMeta` から
/// [`key_env`] で正規化される 5 要素。
///
/// `driver_class` は capture lane の class であり caller が宣言
/// する（backend 名からの推測はしない）。`os_class`/`gpu_class`/
/// `scale_factor`/`font_class` は `EnvironmentMeta` の観測値を
/// 正規化したもので、取得不能な値は `None` のまま。
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenKeyEnv {
    /// Capture driver/lane の class（例: `gpui-real-window-os-capture`）。
    pub driver_class: String,
    /// OS class（`EnvironmentMeta::os` の trim 済み値）。
    pub os_class: String,
    /// GPU class — `gpu.class`（正規化済み）優先、無ければ
    /// `gpu.vendor`。両方無ければ `None`。生 GPU 名
    /// （`gpu.name`）は key に入れない。
    pub gpu_class: Option<String>,
    /// Scale factor（DPI 正規化値。有限かつ正のみ受理）。
    pub scale_factor: f64,
    /// Font class — `font_set` の trim 済み値。空は `None`。
    /// `font_set` は「正規化表現」を期待する field のため、
    /// ここでは空値の排除のみ行い大小文字・内部空白は保持する。
    pub font_class: Option<String>,
}

/// 環境正規化の失敗 — tool error であり verdict ではない。
/// 不正な環境値を `GoldenKey` に混入させないための拒否。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum NormalizeError {
    /// `driver_class` が空（trim 後）。
    EmptyDriverClass,
    /// `EnvironmentMeta::os` が空（trim 後）— OS class を
    /// 構成できない。
    EmptyOsClass,
    /// `scale_factor` が有限かつ正でない（NaN/±inf/0 以下）。
    InvalidScaleFactor {
        /// 拒否された観測値。
        value: f64,
    },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDriverClass => write!(f, "driver_class is empty"),
            Self::EmptyOsClass => write!(f, "environment os class is empty"),
            Self::InvalidScaleFactor { value } => {
                write!(f, "scale_factor {value:?} is not finite and positive")
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

/// `EnvironmentMeta` の観測値を `GoldenKey` の env 側 class へ
/// 正規化する。
///
/// - `driver_class`: caller が宣言する capture lane class
///   （trim、空は `Err(EmptyDriverClass)`）。backend 名
///   （`env.backend`/`backend_detail`）からの推測はしない —
///   lane class は「どの証拠 lane で取った pixel か」の宣言であり
///   backend の実装詳細ではない。
/// - `os_class`: `env.os` の trim 済み値（空は `Err(EmptyOsClass)`）。
/// - `gpu_class`: `env.gpu.class` 優先、無ければ `env.gpu.vendor`、
///   両方無ければ `None`。`gpu.name`（生値）は使わない。
/// - `scale_factor`: `env.scale_factor` が有限かつ正のみ受理
///   （違反は `Err(InvalidScaleFactor)`）。
/// - `font_class`: `env.font_set` の trim 済み値、空は `None`。
///
/// # Errors
/// [`NormalizeError`] when `driver_class`/`env.os` is empty or
/// `env.scale_factor` is not finite and positive.
pub fn key_env(env: &EnvironmentMeta, driver_class: &str) -> Result<GoldenKeyEnv, NormalizeError> {
    let driver_class = driver_class.trim();
    if driver_class.is_empty() {
        return Err(NormalizeError::EmptyDriverClass);
    }
    let os_class = env.os.trim();
    if os_class.is_empty() {
        return Err(NormalizeError::EmptyOsClass);
    }
    if !env.scale_factor.is_finite() || env.scale_factor <= 0.0 {
        return Err(NormalizeError::InvalidScaleFactor {
            value: env.scale_factor,
        });
    }
    // class 優先・vendor fallback — trim 後に空になる値は「無い」
    // ものとして扱い、空白のみの class が有効な vendor を隠さない。
    let gpu_class = env.gpu.as_ref().and_then(|g| {
        [g.class.as_deref(), g.vendor.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|s| !s.is_empty())
            .map(str::to_string)
    });
    let font_class = env
        .font_set
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(GoldenKeyEnv {
        driver_class: driver_class.to_string(),
        os_class: os_class.to_string(),
        gpu_class,
        scale_factor: env.scale_factor,
        font_class,
    })
}

/// `StoryFixture`（#426 code registry）に結合した `GoldenKey` を
/// 構築する。
///
/// `story_id`/`theme` は fixture からそのまま写すため、registry
/// に存在しない story id/theme を key に入れる経路が型で排除
/// される。`state` は consumer の state 軸（semantic walk の
/// state 名を想定するが強制しない — golden state は必ずしも
/// walk state と一致しない）。`revision` は caller が供給する
/// ruviform/アプリ revision（例: `env!("CARGO_PKG_VERSION")`）。
#[must_use]
pub fn key_for_fixture(
    fixture: &StoryFixture,
    state: &str,
    env: &GoldenKeyEnv,
    revision: &str,
) -> GoldenKey {
    GoldenKey {
        story_id: fixture.story_id.to_string(),
        theme: fixture.theme.to_string(),
        state: state.to_string(),
        driver_class: env.driver_class.clone(),
        os_class: env.os_class.clone(),
        gpu_class: env.gpu_class.clone(),
        scale_factor: env.scale_factor,
        font_class: env.font_class.clone(),
        revision: revision.to_string(),
    }
}

/// `registry_mismatches` が報告する 1 件の不整合。
///
/// `field` は `GoldenKey` のどの field が registry と不整合か
/// （`story_id`/`theme`/`state`/`scale_factor`）。`expected` は
/// registry 側の期待（宣言値または許容値一覧）、`actual` は
/// key が持つ値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryMismatch {
    /// 不整合な `GoldenKey` field 名。
    pub field: &'static str,
    /// registry 側の期待値の説明。
    pub expected: String,
    /// key が持つ値。
    pub actual: String,
}

/// `GoldenKey` を `StoryManifest`（#390 `stories.json` JSON
/// registry）と照合し、不整合を列挙する。空 Vec = 整合。
///
/// 検査項目:
///
/// - `story_id`: `manifest.stories` に `id` 一致の entry があるか。
/// - `theme`: `manifest.themes` に含まれるか（story の宣言 theme
///   は 1 つだが #426 matrix は両 theme を走るため、theme 軸は
///   manifest 全体の宣言集合で検査する）。
/// - `state`: 該当 story entry の `states` に含まれるか
///   （story 未登録の場合は state/scale_factor の entry-level
///   検査を skip する — theme は manifest-level のため常に検査）。
/// - `scale_factor`: 該当 story entry の `scaleFactor` に f64
///   完全一致で含まれるか（NaN は一致しない — 正直に mismatch）。
///
/// manifest に無い story（probe 用 key 等）への適用は caller の
/// 判断 — 本関数は強制ではなく監査手段。`driver_class`/`os_class`/
/// `gpu_class`/`font_class`/`revision` は registry の管轄外
/// （driver lane・環境 class・revision は manifest に宣言がない）。
#[must_use]
pub fn registry_mismatches(key: &GoldenKey, manifest: &StoryManifest) -> Vec<RegistryMismatch> {
    let mut mismatches = Vec::new();
    // theme は manifest 全体の宣言集合に対する manifest-level 検査
    // であり entry-level ではない — story_id の成否に依らず常に行う。
    if !manifest.themes.iter().any(|t| t == &key.theme) {
        mismatches.push(RegistryMismatch {
            field: "theme",
            expected: format!("one of: {}", manifest.themes.join(", ")),
            actual: key.theme.clone(),
        });
    }
    let entry = manifest.stories.iter().find(|s| s.id == key.story_id);
    match entry {
        None => mismatches.push(RegistryMismatch {
            field: "story_id",
            expected: format!(
                "one of: {}",
                manifest
                    .stories
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            actual: key.story_id.clone(),
        }),
        Some(entry) => {
            if !entry.states.iter().any(|s| s == &key.state) {
                mismatches.push(RegistryMismatch {
                    field: "state",
                    expected: format!("one of: {}", entry.states.join(", ")),
                    actual: key.state.clone(),
                });
            }
            if !entry.scale_factor.contains(&key.scale_factor) {
                mismatches.push(RegistryMismatch {
                    field: "scale_factor",
                    expected: format!(
                        "one of: {}",
                        entry
                            .scale_factor
                            .iter()
                            .map(|v| format!("{v:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    actual: format!("{:?}", key.scale_factor),
                });
            }
        }
    }
    mismatches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::key::{CapturePath, GpuMeta};

    fn env() -> EnvironmentMeta {
        EnvironmentMeta {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            os_version: None,
            backend: "test-backend".to_string(),
            backend_detail: None,
            gpu: Some(GpuMeta {
                name: Some("Raw GPU Name".to_string()),
                vendor: Some("nvidia".to_string()),
                class: Some("dx11".to_string()),
            }),
            scale_factor: 2.0,
            font_set: Some("inter-v13".to_string()),
            capture: CapturePath::RealWindow,
            ruviform_revision: "0.1.0".to_string(),
            tool_revision: None,
        }
    }

    #[test]
    fn key_env_normalizes_all_fields() {
        let e = env();
        let classes = key_env(&e, "gpui-real-window-os-capture").unwrap();
        assert_eq!(classes.driver_class, "gpui-real-window-os-capture");
        assert_eq!(classes.os_class, "windows");
        // gpu.class が vendor より優先される。
        assert_eq!(classes.gpu_class.as_deref(), Some("dx11"));
        assert_eq!(classes.scale_factor, 2.0);
        assert_eq!(classes.font_class.as_deref(), Some("inter-v13"));
    }

    #[test]
    fn key_env_trims_and_falls_back() {
        let mut e = env();
        e.os = "  linux  ".to_string();
        e.gpu = Some(GpuMeta {
            name: Some("Raw GPU Name".to_string()),
            vendor: Some(" intel ".to_string()),
            class: None,
        });
        e.font_set = Some("   ".to_string());
        let classes = key_env(&e, "  test-driver  ").unwrap();
        assert_eq!(classes.os_class, "linux");
        assert_eq!(classes.driver_class, "test-driver");
        // class が無ければ vendor に fallback。name（生値）は使わない。
        assert_eq!(classes.gpu_class.as_deref(), Some("intel"));
        // 空白のみの class は「無い」ものとして vendor に落ちる。
        e.gpu = Some(GpuMeta {
            name: None,
            vendor: Some("amd".to_string()),
            class: Some("   ".to_string()),
        });
        let classes = key_env(&e, "d").unwrap();
        assert_eq!(classes.gpu_class.as_deref(), Some("amd"));
        // 空白のみの font_set は None。
        assert_eq!(classes.font_class, None);

        e.gpu = None;
        let classes = key_env(&e, "d").unwrap();
        assert_eq!(classes.gpu_class, None);
    }

    #[test]
    fn key_env_rejects_invalid_scale_and_empty_class() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.5] {
            let mut e = env();
            e.scale_factor = bad;
            assert!(
                matches!(
                    key_env(&e, "d"),
                    Err(NormalizeError::InvalidScaleFactor { .. })
                ),
                "scale {bad:?} must be rejected"
            );
        }
        let mut e = env();
        e.os = "   ".to_string();
        assert_eq!(key_env(&e, "d"), Err(NormalizeError::EmptyOsClass));
        assert_eq!(key_env(&env(), "  "), Err(NormalizeError::EmptyDriverClass));
    }

    #[test]
    fn key_for_fixture_binds_registry_ids() {
        let fixture = crate::lab_story::sv_fader_dock_fixture();
        let classes = key_env(&env(), "test-driver").unwrap();
        let key = key_for_fixture(&fixture, "idle", &classes, "rev-1");
        assert_eq!(key.story_id, "LAB-SV-FADER-DOCK");
        assert_eq!(key.theme, "synvelis-dark");
        assert_eq!(key.state, "idle");
        assert_eq!(key.driver_class, "test-driver");
        assert_eq!(key.os_class, "windows");
        assert_eq!(key.gpu_class.as_deref(), Some("dx11"));
        assert_eq!(key.scale_factor, 2.0);
        assert_eq!(key.font_class.as_deref(), Some("inter-v13"));
        assert_eq!(key.revision, "rev-1");
        // canonical/file_stem は決定的。
        assert_eq!(
            key.canonical(),
            key_for_fixture(&fixture, "idle", &classes, "rev-1").canonical()
        );
    }

    fn manifest() -> StoryManifest {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/driver-lab/stories.json"
        );
        crate::driver_lab::load_stories(std::path::Path::new(path)).expect("stories.json loads")
    }

    #[test]
    fn registry_mismatches_empty_for_consistent_key() {
        let manifest = manifest();
        let fixture = crate::lab_story::sv_fader_dock_fixture();
        let classes = GoldenKeyEnv {
            driver_class: "test".to_string(),
            os_class: "test-os".to_string(),
            gpu_class: None,
            scale_factor: 1.0,
            font_class: None,
        };
        let key = key_for_fixture(&fixture, "idle", &classes, "rev");
        assert!(registry_mismatches(&key, &manifest).is_empty());
    }

    #[test]
    fn registry_mismatches_reports_each_drift() {
        let manifest = manifest();
        let fixture = crate::lab_story::sv_fader_dock_fixture();
        let classes = GoldenKeyEnv {
            driver_class: "test".to_string(),
            os_class: "test-os".to_string(),
            gpu_class: None,
            scale_factor: 1.0,
            font_class: None,
        };
        let base = key_for_fixture(&fixture, "idle", &classes, "rev");

        let mut k = base.clone();
        k.story_id = "LAB-NOPE".to_string();
        let m = registry_mismatches(&k, &manifest);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].field, "story_id");

        let mut k = base.clone();
        k.theme = "no-such-theme".to_string();
        let m = registry_mismatches(&k, &manifest);
        assert!(m.iter().any(|x| x.field == "theme"));

        let mut k = base.clone();
        k.state = "no-such-state".to_string();
        let m = registry_mismatches(&k, &manifest);
        assert!(m.iter().any(|x| x.field == "state"));

        let mut k = base.clone();
        k.scale_factor = 3.0;
        let m = registry_mismatches(&k, &manifest);
        assert!(m.iter().any(|x| x.field == "scale_factor"));
    }
}
