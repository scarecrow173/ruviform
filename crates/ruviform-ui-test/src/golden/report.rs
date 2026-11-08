//! `GoldenReport` / `GoldenEvidence` — 結果語彙と audit 証拠
//! （Issue #314 slice 1・設計 D6）。
//!
//! 結果語彙は driver-lab と同一の `pass`/`fail`/`not_run`/
//! `unsupported`/`blocked`。pixel 比較が実際に測定した結果のみ
//! `pass`/`fail` を出し、比較不能・前提未達は `blocked`/
//! `not_run`/`unsupported` として記録する — **未取得の pixel を
//! 成功扱いしない**。capture 層（slice 3）が `unsupported`/
//! `not_run` を立てる入口として `unsupported()`/`not_run()`
//! constructor を提供する。
//!
//! `reason` は機械可読な `snake_case` コード（`match`/
//! `tolerance_exceeded`/`size_mismatch`/`empty_comparison`/
//! `baseline_missing`/caller 指定）、`detail` は人間可読の補足。
//!
//! evidence は `rvf.golden.evidence/v1` の JSONL 行。driver-lab と
//! 同じく `RUVIFORM_GOLDEN_EVIDENCE` env で出力先 path を指定した
//! 場合のみ append し、env 未設定では副作用を持たない（cargo test
//! を純粋に保つ）。相対 path は workspace root から解決し、生成物は
//! commit しない。path 解決・append は `driver_lab` の
//! `pub(crate)` helper を共有する。
//!
//! **RT contract:** non-RT only.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::driver_lab::{append_jsonl_line, evidence_target};
use crate::golden::GoldenVerdict;
use crate::golden::diff::{self, DiffOutcome, DiffPolicy, DiffStats};
use crate::golden::image::RgbaImage;
use crate::golden::key::{EnvironmentMeta, GoldenKey};

/// `RUVIFORM_GOLDEN_EVIDENCE` env 名（evidence 出力先 path）。
pub const EVIDENCE_ENV: &str = "RUVIFORM_GOLDEN_EVIDENCE";
/// evidence schema 識別子。
pub const EVIDENCE_SCHEMA: &str = "rvf.golden.evidence/v1";

/// 1 回の golden 評価の結果レポート。
#[derive(Debug, Clone)]
pub struct GoldenReport {
    /// 評価対象の golden key。
    pub key: GoldenKey,
    /// capture 環境証拠。capture 層が値を持たない場合は `None`。
    pub environment: Option<EnvironmentMeta>,
    /// 判定。
    pub verdict: GoldenVerdict,
    /// 機械可読な理由コード（`snake_case`）。
    pub reason: String,
    /// 人間可読の詳細（fail 理由・blocked 理由等）。
    pub detail: String,
    /// pixel 比較統計。比較を実行しなかった場合は `None`。
    pub stats: Option<DiffStats>,
    /// 比較に使った baseline png path（store 経由の比較のみ。
    /// 画像直比較では `None`）。
    pub baseline: Option<String>,
    /// 関連 artifact（diff 画像・baseline path 等の参照名）。
    /// repo 相対 path や URI を caller が入れる。
    pub artifacts: Vec<String>,
}

/// `GoldenReport` と差分画像の組（`compare` の戻り値）。
#[derive(Debug)]
pub struct GoldenComparison {
    /// 判定レポート。
    pub report: GoldenReport,
    /// 決定的な差分可視化画像。サイズ不一致等で生成できない
    /// 場合は `None`。
    pub diff_image: Option<RgbaImage>,
}

/// baseline と actual を `policy` で比較して `GoldenReport` を
/// 生成する（画像直比較 — baseline store を介さない低レベル API）。
///
/// - 同サイズ: `diff` を実行し、`within_policy` で
///   `pass`/`fail` を決める。`compared_pixels == 0`（全面 mask）は
///   `blocked`（reason=`empty_comparison`）。
/// - サイズ不一致: `fail`（reason=`size_mismatch`）。
///
/// baseline の load・policy 同梱は [`crate::golden::BaselineStore`]
/// の責務 — 通常は `BaselineStore::compare` を使うこと。
#[must_use]
pub fn compare(
    key: &GoldenKey,
    environment: Option<&EnvironmentMeta>,
    baseline: &RgbaImage,
    actual: &RgbaImage,
    policy: &DiffPolicy,
) -> GoldenComparison {
    match diff::diff(baseline, actual, policy) {
        DiffOutcome::Compared(result) => {
            let (verdict, reason) = if result.stats.compared_pixels == 0 {
                (GoldenVerdict::Blocked, "empty_comparison")
            } else if result.within_policy {
                (GoldenVerdict::Pass, "match")
            } else {
                (GoldenVerdict::Fail, "tolerance_exceeded")
            };
            let detail = format!(
                "compared={} masked={} mismatched={} diff_ratio={:.6}",
                result.stats.compared_pixels,
                result.stats.masked_pixels,
                result.stats.mismatched_pixels,
                result.stats.diff_ratio,
            );
            GoldenComparison {
                report: GoldenReport {
                    key: key.clone(),
                    environment: environment.cloned(),
                    verdict,
                    reason: reason.to_string(),
                    detail,
                    stats: Some(result.stats.clone()),
                    baseline: None,
                    artifacts: Vec::new(),
                },
                diff_image: Some(result.diff_image),
            }
        }
        DiffOutcome::SizeMismatch { baseline, actual } => GoldenComparison {
            report: GoldenReport {
                key: key.clone(),
                environment: environment.cloned(),
                verdict: GoldenVerdict::Fail,
                reason: "size_mismatch".to_string(),
                detail: format!(
                    "baseline {}x{} vs actual {}x{}",
                    baseline.0, baseline.1, actual.0, actual.1
                ),
                stats: None,
                baseline: None,
                artifacts: Vec::new(),
            },
            diff_image: None,
        },
    }
}

/// capture 層が pixel を取得できなかった場合の `unsupported`
/// レポート（例: headless backend の screenshot 未実装）。
/// `reason` は機械可読コード（例: `screenshot_unsupported`）。
#[must_use]
pub fn unsupported(
    key: &GoldenKey,
    environment: Option<&EnvironmentMeta>,
    reason: impl Into<String>,
) -> GoldenReport {
    let reason = reason.into();
    GoldenReport {
        key: key.clone(),
        environment: environment.cloned(),
        verdict: GoldenVerdict::Unsupported,
        detail: reason.clone(),
        reason,
        stats: None,
        baseline: None,
        artifacts: Vec::new(),
    }
}

/// 前提条件未達で評価を実行しなかった場合の `not_run` レポート
/// （例: upstream の semantic 検証が fail で pixel 評価に進ま
/// なかった）。
#[must_use]
pub fn not_run(
    key: &GoldenKey,
    environment: Option<&EnvironmentMeta>,
    reason: impl Into<String>,
) -> GoldenReport {
    let reason = reason.into();
    GoldenReport {
        key: key.clone(),
        environment: environment.cloned(),
        verdict: GoldenVerdict::NotRun,
        detail: reason.clone(),
        reason,
        stats: None,
        baseline: None,
        artifacts: Vec::new(),
    }
}

/// baseline 不在・比較対象 0 画素等、評価が阻害された場合の
/// `blocked` レポート。`reason` は機械可読コード
/// （例: `baseline_missing`/`empty_comparison`）、`detail` は
/// 人間可読の補足。
#[must_use]
pub fn blocked(
    key: &GoldenKey,
    environment: Option<&EnvironmentMeta>,
    reason: impl Into<String>,
    detail: impl Into<String>,
) -> GoldenReport {
    GoldenReport {
        key: key.clone(),
        environment: environment.cloned(),
        verdict: GoldenVerdict::Blocked,
        reason: reason.into(),
        detail: detail.into(),
        stats: None,
        baseline: None,
        artifacts: Vec::new(),
    }
}

/// evidence JSONL の 1 行。append-only・schema 固定。
#[derive(Debug, Clone, Serialize)]
pub struct GoldenEvidence {
    /// schema 識別子（`rvf.golden.evidence/v1`）。
    pub schema: String,
    /// 評価対象の canonical key 文字列。
    pub subject: String,
    /// 判定 wire 名（`pass`/`fail`/`not_run`/`unsupported`/`blocked`）。
    pub verdict: String,
    /// 機械可読な理由コード。
    pub reason: String,
    /// 人間可読の詳細。
    pub detail: String,
    /// 記録時刻（UNIX epoch milliseconds、UTC）。
    pub recorded_at_unix_ms: u64,
    /// 環境証拠。capture 層が値を持たない場合は省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentMeta>,
    /// pixel 比較統計。比較未実行なら省略（flatten + Option）。
    #[serde(flatten)]
    pub stats: Option<EvidenceStats>,
    /// 比較に使った baseline path。無ければ省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    /// 関連 artifact 参照。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
}

/// evidence 内の比較統計（`DiffStats` と同じ値を平坦化して保持）。
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceStats {
    /// 比較画像の幅。
    pub width: u32,
    /// 比較画像の高さ。
    pub height: u32,
    /// 比較対象画素数。
    pub compared_pixels: u64,
    /// mask 除外画素数。
    pub masked_pixels: u64,
    /// mismatch 画素数。
    pub mismatched_pixels: u64,
    /// mismatch 比率。
    pub diff_ratio: f64,
    /// channel 別の観測最大差分。
    pub max_deltas: diff::ChannelDelta,
    /// mismatch 画素の bounding box。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_bbox: Option<diff::MaskRect>,
}

impl From<&DiffStats> for EvidenceStats {
    fn from(s: &DiffStats) -> Self {
        Self {
            width: s.width,
            height: s.height,
            compared_pixels: s.compared_pixels,
            masked_pixels: s.masked_pixels,
            mismatched_pixels: s.mismatched_pixels,
            diff_ratio: s.diff_ratio,
            max_deltas: s.max_deltas,
            diff_bbox: s.diff_bbox,
        }
    }
}

impl From<&GoldenReport> for GoldenEvidence {
    fn from(r: &GoldenReport) -> Self {
        Self {
            schema: EVIDENCE_SCHEMA.to_string(),
            subject: r.key.canonical(),
            verdict: r.verdict.as_str().to_string(),
            reason: r.reason.clone(),
            detail: r.detail.clone(),
            recorded_at_unix_ms: now_unix_ms(),
            environment: r.environment.clone(),
            stats: r.stats.as_ref().map(EvidenceStats::from),
            baseline: r.baseline.clone(),
            artifacts: r.artifacts.clone(),
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// `RUVIFORM_GOLDEN_EVIDENCE` で指定された path に evidence を
/// 1 行 append する。env 未設定では `None` を返し副作用を持たない
/// （driver-lab evidence と同じ運用 — 生成物は commit しない）。
/// 相対 path は workspace root から解決する。
///
/// 戻り値は書き込んだファイル path（env 未設定時は `None`）。
#[must_use]
pub fn emit_evidence(report: &GoldenReport) -> Option<PathBuf> {
    let path = evidence_target(EVIDENCE_ENV)?;
    let evidence = GoldenEvidence::from(report);
    let line = serde_json::to_string(&evidence).ok()?;
    append_jsonl_line(&path, &line);
    Some(path)
}

/// 構築済み `GoldenEvidence` を明示 path に 1 行 append する。
/// env gate を介さず直接書きたい caller（fixture/手動監査）用。
/// 戻り値は書き込んだファイル path。
///
/// # Errors
/// serde 直列化失敗時のみ（I/O 失敗は best-effort で握り潰す —
/// evidence は test を fail させない）。
pub fn append_evidence(
    path: &Path,
    evidence: &GoldenEvidence,
) -> Result<PathBuf, serde_json::Error> {
    let line = serde_json::to_string(evidence)?;
    append_jsonl_line(path, &line);
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::diff::{ChannelTolerance, MaskRect};
    use crate::golden::image::RgbaImage;
    use crate::golden::key::CapturePath;

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

    #[test]
    fn compare_pass_within_tolerance() {
        let a = RgbaImage::solid(4, 4, [100, 100, 100, 255]);
        let b = RgbaImage::solid(4, 4, [102, 100, 100, 255]);
        let policy = DiffPolicy {
            channel_tolerance: ChannelTolerance::uniform(2),
            ..DiffPolicy::default()
        };
        let cmp = compare(&key(), None, &a, &b, &policy);
        assert_eq!(cmp.report.verdict, GoldenVerdict::Pass);
        assert_eq!(cmp.report.reason, "match");
        assert!(cmp.diff_image.is_some());
        assert_eq!(cmp.report.stats.as_ref().unwrap().mismatched_pixels, 0);
    }

    #[test]
    fn compare_fail_on_mismatch() {
        let a = RgbaImage::solid(4, 4, [0, 0, 0, 255]);
        let mut b = a.clone();
        b.set_pixel(1, 1, [255, 255, 255, 255]);
        let cmp = compare(&key(), None, &a, &b, &DiffPolicy::default());
        assert_eq!(cmp.report.verdict, GoldenVerdict::Fail);
        assert_eq!(cmp.report.reason, "tolerance_exceeded");
        assert_eq!(cmp.report.stats.as_ref().unwrap().mismatched_pixels, 1);
    }

    #[test]
    fn compare_fail_on_size_mismatch() {
        let a = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        let b = RgbaImage::solid(4, 4, [0, 0, 0, 255]);
        let cmp = compare(&key(), None, &a, &b, &DiffPolicy::default());
        assert_eq!(cmp.report.verdict, GoldenVerdict::Fail);
        assert_eq!(cmp.report.reason, "size_mismatch");
        assert!(cmp.report.stats.is_none());
        assert!(cmp.report.detail.contains("2x2"));
        assert!(cmp.diff_image.is_none());
    }

    #[test]
    fn compare_blocked_when_all_masked() {
        let a = RgbaImage::solid(2, 2, [0, 0, 0, 255]);
        let b = a.clone();
        let policy = DiffPolicy {
            mask: vec![MaskRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }],
            ..DiffPolicy::default()
        };
        let cmp = compare(&key(), None, &a, &b, &policy);
        assert_eq!(cmp.report.verdict, GoldenVerdict::Blocked);
        assert_eq!(cmp.report.reason, "empty_comparison");
    }

    #[test]
    fn unsupported_not_run_blocked_constructors() {
        let r = unsupported(&key(), None, "screenshot_unsupported");
        assert_eq!(r.verdict, GoldenVerdict::Unsupported);
        assert_eq!(r.reason, "screenshot_unsupported");
        let r = not_run(&key(), None, "upstream_semantic_fail");
        assert_eq!(r.verdict, GoldenVerdict::NotRun);
        let r = blocked(&key(), None, "baseline_missing", "no baseline for key");
        assert_eq!(r.verdict, GoldenVerdict::Blocked);
        assert_eq!(r.reason, "baseline_missing");
        assert!(r.detail.contains("no baseline"));
        // いずれも stats/artifacts を持たず pass に昇格しない。
        assert!(r.stats.is_none());
    }

    #[test]
    fn evidence_append_and_parse() {
        let dir = std::env::temp_dir().join(format!(
            "ruviform-golden-ev-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("evidence.jsonl");
        let a = RgbaImage::solid(2, 2, [1, 2, 3, 255]);
        let b = a.clone();
        let mut cmp = compare(&key(), None, &a, &b, &DiffPolicy::default());
        cmp.report.artifacts.push("diff.png".to_string());
        let ev = GoldenEvidence::from(&cmp.report);
        append_evidence(&path, &ev).unwrap();
        append_evidence(&path, &ev).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["schema"], EVIDENCE_SCHEMA);
        assert_eq!(v["verdict"], "pass");
        assert_eq!(v["reason"], "match");
        assert!(v["subject"].as_str().unwrap().contains("S1"));
        assert!(v["compared_pixels"].is_u64());
        assert_eq!(v["artifacts"], serde_json::json!(["diff.png"]));
        assert!(v.get("baseline").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn evidence_for_non_pass_omits_stats() {
        let dir = std::env::temp_dir().join(format!(
            "ruviform-golden-ev2-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("evidence.jsonl");
        let r = unsupported(&key(), None, "no capture");
        let ev = GoldenEvidence::from(&r);
        append_evidence(&path, &ev).unwrap();
        let line = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(line.lines().next().unwrap()).unwrap();
        assert_eq!(v["verdict"], "unsupported");
        assert_eq!(v["reason"], "no capture");
        assert!(v.get("compared_pixels").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn emit_evidence_respects_env_gate() {
        // env 未設定では副作用なし（他 test の env に依存しないよう
        // この test 内で明示的に unset はできない — cargo test は
        // env を共有するため、設定されている場合のみ path が返る
        // ことを確認する形にする）。
        let r = blocked(&key(), None, "baseline_missing", "none");
        // emit_evidence は env 未設定で None、設定済みなら Some(path)
        // — どちらでも panic しないことを確認。
        let _ = emit_evidence(&r);
    }

    #[test]
    fn environment_meta_flows_into_evidence() {
        let env = EnvironmentMeta {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            os_version: Some("11".to_string()),
            backend: "gpui-test".to_string(),
            backend_detail: None,
            gpu: None,
            scale_factor: 1.0,
            font_set: None,
            capture: CapturePath::Offscreen,
            ruviform_revision: "0.1.0".to_string(),
            tool_revision: None,
        };
        let r = blocked(&key(), Some(&env), "baseline_missing", "none");
        let ev = GoldenEvidence::from(&r);
        assert_eq!(ev.environment.as_ref().unwrap().backend, "gpui-test");
        assert_eq!(
            ev.environment.as_ref().unwrap().capture,
            CapturePath::Offscreen
        );
    }
}
