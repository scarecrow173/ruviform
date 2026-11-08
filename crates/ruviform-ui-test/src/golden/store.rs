//! `BaselineStore` — repo 内 baseline PNG + sidecar の配置・
//! candidate/promote・比較（Issue #314 slice 1・設計 D5）。
//!
//! レイアウト（`root` は caller が repo 内 fixture dir を指す）:
//!
//! ```text
//! <root>/<story_id>/<theme>/<state>/<file_stem>.png
//! <root>/<story_id>/<theme>/<state>/<file_stem>.json
//! ```
//!
//! `<story_id>`/`<theme>`/`<state>` は `GoldenKey::path_segments()`、
//! `<file_stem>` は `GoldenKey::file_stem()` — path segment は
//! sanitize 済みで traversal を含まない。
//!
//! **承認境界（candidate → promote の 2 相）:**
//!
//! - [`BaselineStore::write_candidate`] は staging dir
//!   （慣例 `target/golden/`）に candidate PNG+sidecar を書くだけで
//!   baseline を触らない。candidate sidecar の `approval` は未設定。
//! - [`BaselineStore::promote`] が staged candidate を baseline path
//!   へ書き込む唯一の経路であり、[`BaselineApproval`]（承認者・
//!   理由）を必須とする。人間のレビュー（diff 画像の目視 + PR での
//!   fixture 差分承認）を経て初めて baseline が更新される。
//! - **auto-update 経路（`UPDATE_GOLDEN` 等の env）は存在しない** —
//!   baseline 更新の approval/audit 必須という Issue 契約を機械的に
//!   強制するため。
//! - sidecar は `key`・`environment`・`policy`（承認時の
//!   `DiffPolicy`）・画像メタデータを保持する。**policy を baseline
//!   に同梱する**のは、承認時の比較条件を baseline と一体で固定し、
//!   比較時に黙って緩められないようにするため。
//!
//! 書き込みは `<name>.tmp-<pid>` に書いてから rename する
//! （同一 dir 内 rename = 同一 volume で atomic）。`read` は
//! sidecar の key・schema・画像 fingerprint を png と照合し、
//! 不整合を [`StoreError`] として拒否する — 壊れた baseline を
//! 比較に使わせない。
//!
//! **RT contract:** non-RT only.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::golden::GoldenError;
use crate::golden::diff::DiffPolicy;
use crate::golden::image::{GoldenImageError, RgbaImage};
use crate::golden::key::{EnvironmentMeta, GoldenKey};
use crate::golden::report::{self, GoldenComparison};

/// sidecar の schema 識別子。
pub const BASELINE_SIDECAR_SCHEMA: &str = "rvf.golden.baseline/v1";

/// baseline/candidate sidecar（`<file_stem>.json`）。
///
/// candidate（staging）では `approval` が `None`、promote 済み
/// baseline では `Some` — schema は共通で、承認の有無が状態を表す。
pub struct BaselineSidecar {
    /// sidecar schema。
    pub schema: String,
    /// この baseline が属する golden key。
    pub key: GoldenKey,
    /// capture 時の環境証拠。記録が無い baseline は `None`。
    pub environment: Option<EnvironmentMeta>,
    /// 承認時に固定された比較 policy。
    pub policy: DiffPolicy,
    /// 画像メタデータ。
    pub image: BaselineImageMeta,
    /// baseline 配置の承認記録。candidate では `None`。
    pub approval: Option<BaselineApproval>,
}

/// sidecar 内の画像メタデータ。
pub struct BaselineImageMeta {
    /// 幅。
    pub width: u32,
    /// 高さ。
    pub height: u32,
    /// PNG エンコード後の byte 数。
    pub png_bytes: u64,
    /// 画素内容の決定的 fingerprint（FNV-1a 64bit of RGBA8 bytes）。
    /// baseline 更新時の「何が変わったか」の照合用。
    pub pixel_fingerprint: String,
}

/// baseline 配置（promote）の承認記録 — `promote` で必須。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineApproval {
    /// 承認者の識別子（人間/エージェント名・レビュー ID 等）。
    pub approved_by: String,
    /// 承認理由・レビュー参照。任意。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `BaselineStore` が返す baseline/candidate 一式。
pub struct Baseline {
    /// decode 済み RGBA8 画像。
    pub image: RgbaImage,
    /// sidecar の内容。
    pub sidecar: BaselineSidecar,
}

/// baseline store の失敗。
#[non_exhaustive]
#[derive(Debug)]
pub enum StoreError {
    /// ファイルシステム I/O 失敗。
    Io(std::io::Error),
    /// baseline PNG の decode/encode 失敗。
    Image(GoldenImageError),
    /// sidecar JSON の parse 失敗・schema 不一致。
    CorruptSidecar {
        /// sidecar path。
        path: PathBuf,
        /// 失敗理由。
        reason: String,
    },
    /// sidecar の key が要求された key と不一致（別 baseline の
    /// sidecar が置かれている等）。
    KeyMismatch {
        /// 要求された key の canonical 文字列。
        expected: String,
        /// sidecar に記録された key の canonical 文字列。
        found: String,
    },
    /// png は存在するが sidecar が無い/読めない（不完全な baseline
    /// を黙って受理しない）。
    MissingSidecar {
        /// png path。
        path: PathBuf,
    },
    /// promote 対象の candidate が staging に存在しない。
    CandidateMissing {
        /// 要求された key の canonical 文字列。
        key: String,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Image(e) => write!(f, "image: {e}"),
            Self::CorruptSidecar { path, reason } => {
                write!(f, "corrupt sidecar {}: {reason}", path.display())
            }
            Self::KeyMismatch { expected, found } => {
                write!(f, "key mismatch: expected {expected}, found {found}")
            }
            Self::MissingSidecar { path } => {
                write!(
                    f,
                    "baseline png exists but sidecar missing: {}",
                    path.display()
                )
            }
            Self::CandidateMissing { key } => {
                write!(f, "no staged candidate for key {key}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<GoldenImageError> for StoreError {
    fn from(e: GoldenImageError) -> Self {
        Self::Image(e)
    }
}

/// repo 内 baseline store の root。
pub struct BaselineStore {
    root: PathBuf,
}

impl BaselineStore {
    /// `root` を baseline ルートとして store を構築する。
    /// `root` が存在しなくてもよい — `promote` が必要に応じて作る。
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// baseline ルート。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// baseline の `(png_path, sidecar_path)` を返す。
    #[must_use]
    pub fn paths(&self, key: &GoldenKey) -> (PathBuf, PathBuf) {
        self.paths_under(&self.root, key)
    }

    /// `dir` 配下の `(png_path, sidecar_path)`。candidate staging
    /// でも同じ layout を使う。
    fn paths_under(&self, dir: &Path, key: &GoldenKey) -> (PathBuf, PathBuf) {
        let mut d = dir.to_path_buf();
        for seg in key.path_segments() {
            d.push(seg);
        }
        let stem = key.file_stem();
        (
            d.join(format!("{stem}.png")),
            d.join(format!("{stem}.json")),
        )
    }

    /// baseline を読む。png が存在しなければ `Ok(None)`。
    ///
    /// png があるのに sidecar が無い/壊れている/別 key の場合は
    /// `Err` — 不完全・不整合な baseline を比較に使わせない。
    ///
    /// # Errors
    /// [`StoreError`] on I/O, decode, or integrity failures.
    pub fn read(&self, key: &GoldenKey) -> Result<Option<Baseline>, StoreError> {
        self.read_under(&self.root, key)
    }

    /// `dir` 配下の baseline/candidate を読む（`read` と同一規則）。
    fn read_under(&self, dir: &Path, key: &GoldenKey) -> Result<Option<Baseline>, StoreError> {
        let (png_path, sidecar_path) = self.paths_under(dir, key);
        if !png_path.exists() {
            return Ok(None);
        }
        let png_bytes = fs::read(&png_path)?;
        let sidecar_bytes = fs::read(&sidecar_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StoreError::MissingSidecar {
                    path: png_path.clone(),
                }
            } else {
                StoreError::Io(e)
            }
        })?;
        let sidecar = parse_sidecar(&sidecar_path, &sidecar_bytes)?;
        if sidecar.key.canonical() != key.canonical() {
            return Err(StoreError::KeyMismatch {
                expected: key.canonical(),
                found: sidecar.key.canonical(),
            });
        }
        let image = RgbaImage::from_png(&png_bytes)?;
        // sidecar 記録と実画像の整合を検証する。
        let meta = &sidecar.image;
        if meta.width != image.width()
            || meta.height != image.height()
            || meta.pixel_fingerprint != fingerprint(image.pixels())
        {
            return Err(StoreError::CorruptSidecar {
                path: sidecar_path,
                reason: "image metadata/fingerprint does not match png content".to_string(),
            });
        }
        Ok(Some(Baseline { image, sidecar }))
    }

    /// candidate（PNG+sidecar、approval 無し）を `staging_root` 配下に
    /// 書き込む。baseline には触れない — 人間レビュー（diff 画像の
    /// 目視・PR 差分）を経て [`Self::promote`] で初めて baseline に
    /// なる。
    ///
    /// # Errors
    /// [`StoreError`] on I/O or encode failure.
    pub fn write_candidate(
        &self,
        staging_root: &Path,
        key: &GoldenKey,
        image: &RgbaImage,
        environment: Option<&EnvironmentMeta>,
        policy: &DiffPolicy,
    ) -> Result<BaselineSidecar, StoreError> {
        self.write_under(staging_root, key, image, environment, policy, None)
    }

    /// staging の candidate を baseline root へ promote する —
    /// baseline を更新する**唯一の経路**。`approval` は必須で、
    /// sidecar に承認記録として残る。
    ///
    /// candidate が無い・壊れている場合は `Err`（baseline は変更
    /// されない — 読み取り検証を先に行う）。
    ///
    /// # Errors
    /// [`StoreError`] on missing/corrupt candidate or I/O failure.
    pub fn promote(
        &self,
        staging_root: &Path,
        key: &GoldenKey,
        approval: &BaselineApproval,
    ) -> Result<BaselineSidecar, StoreError> {
        let candidate =
            self.read_under(staging_root, key)?
                .ok_or_else(|| StoreError::CandidateMissing {
                    key: key.canonical(),
                })?;
        self.write_under(
            &self.root,
            key,
            &candidate.image,
            candidate.sidecar.environment.as_ref(),
            &candidate.sidecar.policy,
            Some(approval),
        )
    }

    /// `dir` 配下に PNG+sidecar を書き込む内部実装。
    /// `approval` は candidate では `None`、promote 済み baseline
    /// では `Some`。
    fn write_under(
        &self,
        dir: &Path,
        key: &GoldenKey,
        image: &RgbaImage,
        environment: Option<&EnvironmentMeta>,
        policy: &DiffPolicy,
        approval: Option<&BaselineApproval>,
    ) -> Result<BaselineSidecar, StoreError> {
        let (png_path, sidecar_path) = self.paths_under(dir, key);
        if let Some(parent) = png_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let png_bytes = image.to_png()?;
        let sidecar = BaselineSidecar {
            schema: BASELINE_SIDECAR_SCHEMA.to_string(),
            key: key.clone(),
            environment: environment.cloned(),
            policy: policy.clone(),
            image: BaselineImageMeta {
                width: image.width(),
                height: image.height(),
                png_bytes: png_bytes.len() as u64,
                pixel_fingerprint: fingerprint(image.pixels()),
            },
            approval: approval.cloned(),
        };
        let sidecar_json =
            serde_json::to_vec_pretty(&SidecarWire::from(&sidecar)).map_err(|e| {
                StoreError::CorruptSidecar {
                    path: sidecar_path.clone(),
                    reason: format!("serialize: {e}"),
                }
            })?;
        atomic_write(&png_path, &png_bytes)?;
        atomic_write(&sidecar_path, &sidecar_json)?;
        Ok(sidecar)
    }

    /// baseline と actual を承認済み sidecar の `policy` で比較し、
    /// `GoldenReport`（+ 差分画像）を返す。
    ///
    /// - baseline 不在 → verdict `blocked`（reason=`baseline_missing`）。
    /// - baseline 破損・key 不一致 → `Err(GoldenError::Store)`
    ///   （比較不能は verdict ではなく tool error）。
    /// - サイズ不一致 → `fail`（reason=`size_mismatch`）。
    /// - `compared_pixels == 0`（全面 mask）→ `blocked`
    ///   （reason=`empty_comparison`）。
    ///
    /// 戻り値の `report.baseline` に baseline png path を記録する。
    /// 差分画像の artifact 保存は caller の責務。
    ///
    /// # Errors
    /// [`GoldenError::Store`] on store read/integrity failures.
    pub fn compare(
        &self,
        key: &GoldenKey,
        actual: &RgbaImage,
        environment: Option<&EnvironmentMeta>,
    ) -> Result<GoldenComparison, GoldenError> {
        let Some(baseline) = self.read(key)? else {
            let mut report = report::blocked(
                key,
                environment,
                "baseline_missing",
                format!("no baseline for key {}", key.canonical()),
            );
            report.baseline = Some(self.paths(key).0.display().to_string());
            return Ok(GoldenComparison {
                report,
                diff_image: None,
            });
        };
        let mut cmp = report::compare(
            key,
            environment,
            &baseline.image,
            actual,
            &baseline.sidecar.policy,
        );
        cmp.report.baseline = Some(self.paths(key).0.display().to_string());
        Ok(cmp)
    }
}

/// sidecar の wire 表現（`Option` の skip を制御するため
/// `BaselineSidecar` とは別に serde 型を持つ）。
#[derive(Serialize)]
struct SidecarWire<'a> {
    schema: &'a str,
    key: &'a GoldenKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<&'a EnvironmentMeta>,
    policy: &'a DiffPolicy,
    image: ImageMetaWire<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval: Option<&'a BaselineApproval>,
}

#[derive(Serialize)]
struct ImageMetaWire<'a> {
    width: u32,
    height: u32,
    png_bytes: u64,
    pixel_fingerprint: &'a str,
}

impl<'a> From<&'a BaselineSidecar> for SidecarWire<'a> {
    fn from(s: &'a BaselineSidecar) -> Self {
        Self {
            schema: &s.schema,
            key: &s.key,
            environment: s.environment.as_ref(),
            policy: &s.policy,
            image: ImageMetaWire {
                width: s.image.width,
                height: s.image.height,
                png_bytes: s.image.png_bytes,
                pixel_fingerprint: &s.image.pixel_fingerprint,
            },
            approval: s.approval.as_ref(),
        }
    }
}

#[derive(Deserialize)]
struct SidecarWireOwned {
    schema: String,
    key: GoldenKey,
    #[serde(default)]
    environment: Option<EnvironmentMeta>,
    policy: DiffPolicy,
    image: ImageMetaWireOwned,
    #[serde(default)]
    approval: Option<BaselineApproval>,
}

#[derive(Deserialize)]
struct ImageMetaWireOwned {
    width: u32,
    height: u32,
    png_bytes: u64,
    pixel_fingerprint: String,
}

fn parse_sidecar(path: &Path, bytes: &[u8]) -> Result<BaselineSidecar, StoreError> {
    let wire: SidecarWireOwned =
        serde_json::from_slice(bytes).map_err(|e| StoreError::CorruptSidecar {
            path: path.to_path_buf(),
            reason: format!("json parse: {e}"),
        })?;
    if wire.schema != BASELINE_SIDECAR_SCHEMA {
        return Err(StoreError::CorruptSidecar {
            path: path.to_path_buf(),
            reason: format!("unknown schema `{}`", wire.schema),
        });
    }
    Ok(BaselineSidecar {
        schema: wire.schema,
        key: wire.key,
        environment: wire.environment,
        policy: wire.policy,
        image: BaselineImageMeta {
            width: wire.image.width,
            height: wire.image.height,
            png_bytes: wire.image.png_bytes,
            pixel_fingerprint: wire.image.pixel_fingerprint,
        },
        approval: wire.approval,
    })
}

/// `<name>.tmp-<pid>` に書いて rename する簡易 atomic write。
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// RGBA8 bytes の FNV-1a 64bit fingerprint（`"fnv1a:<16hex>"`）。
/// baseline 更新の差分監査用の決定的ハッシュ — 衝突耐性は
/// 監査補助用途に十分（暗号学的用途ではない）。
fn fingerprint(pixels: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in pixels {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a:{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::diff::ChannelTolerance;
    use crate::golden::key::CapturePath;
    use crate::golden::{GoldenVerdict, diff};

    fn key() -> GoldenKey {
        GoldenKey {
            story_id: "S1".to_string(),
            theme: "dark".to_string(),
            state: "idle".to_string(),
            driver_class: "test".to_string(),
            os_class: "test-os".to_string(),
            gpu_class: None,
            scale_factor: 1.0,
            font_class: None,
            revision: "rev1".to_string(),
        }
    }

    fn env() -> EnvironmentMeta {
        EnvironmentMeta {
            os: "test-os".to_string(),
            arch: "test-arch".to_string(),
            os_version: None,
            backend: "test".to_string(),
            backend_detail: None,
            gpu: None,
            scale_factor: 1.0,
            font_set: None,
            capture: CapturePath::Unknown,
            ruviform_revision: "0.0.0".to_string(),
            tool_revision: None,
        }
    }

    fn approval() -> BaselineApproval {
        BaselineApproval {
            approved_by: "test-reviewer".to_string(),
            note: Some("initial baseline".to_string()),
        }
    }

    fn policy() -> DiffPolicy {
        DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(2),
            mask: Vec::new(),
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ruviform-golden-store-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = temp_dir("missing");
        let store = BaselineStore::new(&dir);
        assert!(store.read(&key()).unwrap().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn candidate_does_not_touch_baseline_until_promote() {
        let base = temp_dir("base");
        let staging = temp_dir("staging");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(4, 3, [10, 20, 30, 255]);

        // candidate write → baseline には何も無い
        let cand = store
            .write_candidate(&staging, &key(), &img, Some(&env()), &policy())
            .unwrap();
        assert_eq!(cand.schema, BASELINE_SIDECAR_SCHEMA);
        assert!(cand.approval.is_none(), "candidate must have no approval");
        assert!(store.read(&key()).unwrap().is_none());
        // staging 側には candidate がある
        let staged = store.read_under(&staging, &key()).unwrap().unwrap();
        assert_eq!(staged.image, img);
        assert_eq!(staged.sidecar.policy, policy());

        // promote → baseline に approval 付きで配置される
        let promoted = store.promote(&staging, &key(), &approval()).unwrap();
        assert_eq!(promoted.approval, Some(approval()));
        let baseline = store.read(&key()).unwrap().expect("baseline exists");
        assert_eq!(baseline.image, img);
        assert_eq!(baseline.sidecar.approval, Some(approval()));
        assert_eq!(baseline.sidecar.environment, Some(env()));
        assert_eq!(baseline.sidecar.policy, policy());

        // layout: <root>/S1/dark/idle/<stem>.png + .json
        let (png_path, sidecar_path) = store.paths(&key());
        assert!(
            png_path
                .parent()
                .is_some_and(|p| p.ends_with(Path::new("S1").join("dark").join("idle")))
        );
        assert!(png_path.exists());
        assert!(sidecar_path.exists());
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn promote_without_candidate_fails_and_keeps_baseline() {
        let base = temp_dir("base2");
        let staging = temp_dir("staging2");
        let store = BaselineStore::new(&base);
        assert!(matches!(
            store.promote(&staging, &key(), &approval()),
            Err(StoreError::CandidateMissing { .. })
        ));
        assert!(store.read(&key()).unwrap().is_none());
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn promote_updates_existing_baseline_with_new_approval() {
        let base = temp_dir("base3");
        let staging = temp_dir("staging3");
        let store = BaselineStore::new(&base);
        let img1 = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        let img2 = RgbaImage::solid(2, 2, [255, 255, 255, 255]);
        store
            .write_candidate(&staging, &key(), &img1, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        // 2 回目の candidate → promote で baseline 更新
        store
            .write_candidate(&staging, &key(), &img2, None, &policy())
            .unwrap();
        let new_approval = BaselineApproval {
            approved_by: "second-reviewer".to_string(),
            note: None,
        };
        store.promote(&staging, &key(), &new_approval).unwrap();
        let baseline = store.read(&key()).unwrap().unwrap();
        assert_eq!(baseline.image, img2);
        assert_eq!(baseline.sidecar.approval, Some(new_approval));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn different_env_keys_coexist() {
        let base = temp_dir("base4");
        let staging = temp_dir("staging4");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(1, 1, [1, 2, 3, 255]);
        let mut k2 = key();
        k2.scale_factor = 2.0;
        store
            .write_candidate(&staging, &key(), &img, None, &policy())
            .unwrap();
        store
            .write_candidate(&staging, &k2, &img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        store.promote(&staging, &k2, &approval()).unwrap();
        assert!(store.read(&key()).unwrap().is_some());
        assert!(store.read(&k2).unwrap().is_some());
        // stem が異なるので別ファイル
        assert_ne!(store.paths(&key()).0, store.paths(&k2).0);
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn corrupted_png_rejected() {
        let base = temp_dir("base5");
        let staging = temp_dir("staging5");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        store
            .write_candidate(&staging, &key(), &img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        let (png_path, _) = store.paths(&key());
        fs::write(&png_path, b"garbage").unwrap();
        assert!(matches!(store.read(&key()), Err(StoreError::Image(_))));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn missing_sidecar_rejected() {
        let base = temp_dir("base6");
        let staging = temp_dir("staging6");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        store
            .write_candidate(&staging, &key(), &img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        let (_, sidecar_path) = store.paths(&key());
        fs::remove_file(&sidecar_path).unwrap();
        assert!(matches!(
            store.read(&key()),
            Err(StoreError::MissingSidecar { .. })
        ));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn key_mismatch_rejected() {
        let base = temp_dir("base7");
        let staging = temp_dir("staging7");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        store
            .write_candidate(&staging, &key(), &img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        // sidecar の key を改ざんする
        let (_, sc) = store.paths(&key());
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&sc).unwrap()).unwrap();
        v["key"]["state"] = serde_json::json!("tampered");
        fs::write(&sc, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        assert!(matches!(
            store.read(&key()),
            Err(StoreError::KeyMismatch { .. })
        ));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn tampered_pixels_rejected_by_fingerprint() {
        let base = temp_dir("base8");
        let staging = temp_dir("staging8");
        let store = BaselineStore::new(&base);
        let img = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        store
            .write_candidate(&staging, &key(), &img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        // PNG を別画像で置き換える（sidecar はそのまま）→ fingerprint 不一致
        let other = RgbaImage::solid(2, 2, [255, 255, 255, 255]);
        let (png_path, _) = store.paths(&key());
        fs::write(&png_path, other.to_png().unwrap()).unwrap();
        assert!(matches!(
            store.read(&key()),
            Err(StoreError::CorruptSidecar { .. })
        ));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn compare_uses_baseline_policy_and_records_baseline_path() {
        let base = temp_dir("base9");
        let staging = temp_dir("staging9");
        let store = BaselineStore::new(&base);
        // baseline は tolerance=2 で promote される
        let baseline_img = RgbaImage::solid(4, 4, [100, 100, 100, 255]);
        store
            .write_candidate(&staging, &key(), &baseline_img, None, &policy())
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();

        // actual は +1 ずれ → tolerance 2 内で pass
        let actual = RgbaImage::solid(4, 4, [101, 100, 100, 255]);
        let cmp = store.compare(&key(), &actual, None).unwrap();
        assert_eq!(cmp.report.verdict, GoldenVerdict::Pass);
        assert!(cmp.report.baseline.is_some());
        assert!(cmp.report.baseline.as_ref().unwrap().ends_with(".png"));

        // +10 ずれ → fail（sidecar の policy=2 が使われる）
        let actual2 = RgbaImage::solid(4, 4, [110, 100, 100, 255]);
        let cmp2 = store.compare(&key(), &actual2, None).unwrap();
        assert_eq!(cmp2.report.verdict, GoldenVerdict::Fail);
        assert_eq!(cmp2.report.reason, "tolerance_exceeded");

        // baseline 不在 key → blocked
        let mut other = key();
        other.state = "focus".to_string();
        let cmp3 = store.compare(&other, &actual, None).unwrap();
        assert_eq!(cmp3.report.verdict, GoldenVerdict::Blocked);
        assert_eq!(cmp3.report.reason, "baseline_missing");
        assert!(cmp3.diff_image.is_none());

        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn compare_size_mismatch_fails() {
        let base = temp_dir("base10");
        let staging = temp_dir("staging10");
        let store = BaselineStore::new(&base);
        store
            .write_candidate(
                &staging,
                &key(),
                &RgbaImage::solid(2, 2, [0; 4]),
                None,
                &policy(),
            )
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        let cmp = store
            .compare(&key(), &RgbaImage::solid(4, 4, [0; 4]), None)
            .unwrap();
        assert_eq!(cmp.report.verdict, GoldenVerdict::Fail);
        assert_eq!(cmp.report.reason, "size_mismatch");
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn sidecar_records_policy_for_audit() {
        let base = temp_dir("base11");
        let staging = temp_dir("staging11");
        let store = BaselineStore::new(&base);
        store
            .write_candidate(
                &staging,
                &key(),
                &RgbaImage::solid(1, 1, [0; 4]),
                None,
                &policy(),
            )
            .unwrap();
        store.promote(&staging, &key(), &approval()).unwrap();
        let baseline = store.read(&key()).unwrap().unwrap();
        // sidecar の policy が承認時の値を保持している
        assert_eq!(baseline.sidecar.policy.channel_tolerance.red, 2);
        // diff 関数も policy を直接受け取る（store 経由でない低レベル確認）
        let a = RgbaImage::solid(1, 1, [0, 0, 0, 255]);
        let b = RgbaImage::solid(1, 1, [3, 0, 0, 255]);
        let outcome = diff(&a, &b, &baseline.sidecar.policy);
        assert!(matches!(outcome, crate::golden::DiffOutcome::Compared(_)));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&staging);
    }
}
