//! Issue #314 Pixel Golden — headless baseline 比較 utility コア
//! （slice 1・設計 `docs/designs/issue-314-golden-core.md`）。
//!
//! 実際に得られた pixel screenshot（RGBA8）を baseline と比較し、
//! 許容差・region mask・差分画像・証拠を扱う driver-neutral な
//! utility。**capture（pixel の取得）は対象外** — 本 module は
//! decode 済みの画像と `proto::Screenshot` wire 型の変換、比較、
//! baseline store（candidate → promote の承認境界）、証拠出力を
//! 担う。`GpuiObserver::drawn()` の `DrawPlan` は semantic/描画計画
//! であり pixel ではない — pixel の `Pass` は semantic oracle の
//! `Pass` を意味しない。
//!
//! - [`key`]: `GoldenKey`（baseline 同一性）と `EnvironmentMeta`
//!   （capture 環境の証拠）。
//! - [`image`]: `RgbaImage`（RGBA8）と PNG / `Screenshot` decode。
//! - [`diff`]: `DiffPolicy`（per-channel 許容差 + mask）と純粋関数
//!   `diff`（統計 + 差分画像 + policy 判定）。
//! - [`store`]: `BaselineStore`（repo 内 baseline 配置・
//!   candidate/promote 承認境界・sidecar policy 同梱）。
//! - [`normalize`]: `EnvironmentMeta` → `GoldenKey` env class の
//!   正規化（`key_env`）と #426/#390 registry への結合・整合検査
//!   （`key_for_fixture`/`registry_mismatches`）。
//! - [`report`]: `GoldenReport` / `GoldenEvidence`（結果語彙と
//!   `RUVIFORM_GOLDEN_EVIDENCE` 環境変数ゲートの evidence JSONL）。
//!
//! 判定語彙は driver-lab と同じ `pass`/`fail`/`not_run`/`unsupported`/
//! `blocked` の 5 値。pixel 比較が測定した結果のみ `pass`/`fail` を
//! 出し、baseline 不在や比較不能は `blocked`、capture 層の未対応・
//! 未実行は `unsupported`/`not_run` として記録する — 未取得を
//! 成功扱いしない。baseline 更新は `write_candidate` → 人間レビュー
//! → `promote`（承認記録必須）の 2 相で、自動更新経路は存在しない。
//!
//! **RT contract:** non-RT only.

pub mod diff;
pub mod image;
pub mod key;
pub mod normalize;
pub mod report;
pub mod store;

use std::fmt;

pub use diff::{
    ChannelDelta, ChannelTolerance, DiffOutcome, DiffPolicy, DiffResult, DiffStats, MaskRect, diff,
};
pub use image::{GoldenImageError, RgbaImage};
pub use key::{CapturePath, EnvironmentMeta, GoldenKey, GpuMeta};
pub use normalize::{
    GoldenKeyEnv, NormalizeError, RegistryMismatch, key_env, key_for_fixture, registry_mismatches,
};
pub use report::{
    EVIDENCE_ENV, EVIDENCE_LANE, EVIDENCE_SCHEMA, EvidenceStats, GoldenComparison, GoldenEvidence,
    GoldenReport, append_evidence, blocked, compare, emit_evidence, not_run, unsupported,
};
pub use store::{
    BASELINE_SIDECAR_SCHEMA, Baseline, BaselineApproval, BaselineImageMeta, BaselineSidecar,
    BaselineStore, StoreError,
};

/// Golden 判定語彙 — driver-lab（`driver_lab::VERDICT_VOCABULARY`）
/// と同一の 5 値。`not_run`/`unsupported`/`blocked` は決して `pass`
/// に昇格しない。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoldenVerdict {
    /// baseline との pixel 差分が policy 内。
    Pass,
    /// pixel 差分が policy を超過、またはサイズ不一致。
    Fail,
    /// 前提条件未達で評価を実行しなかった（capture 層が使用）。
    NotRun,
    /// 環境/backend が当該評価をサポートしない（capture 層が使用）。
    Unsupported,
    /// baseline 不在・比較対象 0 画素など評価が阻害された。
    Blocked,
}

impl GoldenVerdict {
    /// Wire 名（`pass`/`fail`/`not_run`/`unsupported`/`blocked`）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotRun => "not_run",
            Self::Unsupported => "unsupported",
            Self::Blocked => "blocked",
        }
    }
}

/// golden utility の tool レベル失敗 — 比較結果（verdict）ではなく
/// harness/codec/store の異常。decode 失敗・IO 失敗・破損 store を
/// `pass`/`fail` に見せかけない。
#[non_exhaustive]
#[derive(Debug)]
pub enum GoldenError {
    /// 画像 decode/encode の失敗。
    Image(GoldenImageError),
    /// baseline store の読み書き・整合性失敗。
    Store(StoreError),
    /// その他の I/O 失敗。
    Io(std::io::Error),
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(e) => write!(f, "image: {e}"),
            Self::Store(e) => write!(f, "store: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for GoldenError {}

impl From<GoldenImageError> for GoldenError {
    fn from(e: GoldenImageError) -> Self {
        Self::Image(e)
    }
}

impl From<StoreError> for GoldenError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<std::io::Error> for GoldenError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
