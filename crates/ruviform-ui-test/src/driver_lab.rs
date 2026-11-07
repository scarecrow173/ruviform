//! Issue #390 driver contract lab — UI-DVR case manifest と
//! Native Component Lab story registry の型・loader・整合性検査。
//!
//! `cases.json` / `stories.json` が semantic truth（oracle・binding・
//! verdict 語彙の正本）であり、この module は構造検証と実行
//! binding の存在確認を行う。verdict 語彙は
//! `pass`/`fail`/`not_run`/`unsupported`/`blocked` の 5 値で、
//! `not_run`/`unsupported`/`blocked` は決して `pass` に昇格しない。

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 判定語彙（spec R6）。宣言順が正本。
/// consumer-verify lane（`consumer-verify-tuple/v1`）の語彙は
/// `pass/fail/not_run/unsupported/skipped` で `blocked` を持たない —
/// lane 接続時は `blocked` を `not_run` へ写像する（README/設計参照）。
pub const VERDICT_VOCABULARY: [&str; 5] = ["pass", "fail", "not_run", "unsupported", "blocked"];

/// driver layer 語彙。manifest/story 共通。
pub const LAYER_VOCABULARY: [&str; 5] = ["boundary", "fake", "gpui", "real-os", "headless-ts"];

/// UI-DVR case manifest（`fixtures/driver-lab/cases.json`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseManifest {
    /// schemaVersion。
    #[serde(rename = "schemaVersion")]
    pub schema_version: u64,
    /// manifest 識別子（`rvf.driver-lab.cases/v1`）。
    pub manifest: String,
    /// 発行元 Issue URL。
    #[serde(default)]
    pub issue: Option<String>,
    /// 宣言 verdict 語彙。
    pub vocabulary: Vec<String>,
    /// 宣言 layer（`boundary`/`fake`/`gpui`/`real-os`/`headless-ts`）。
    pub layers: Vec<String>,
    /// 必須 Case ID 集合（UI-DVR-01〜08 の完全性を pin する）。
    #[serde(rename = "requiredCases", default)]
    pub required_cases: Vec<String>,
    /// case 一覧。
    pub cases: Vec<CaseEntry>,
}

/// 1 case。oracle と layer 別 binding を持つ。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseEntry {
    /// Case ID（`UI-DVR-\d{2}`）。
    pub id: String,
    /// 件名。
    pub title: String,
    /// semantic oracle（期待する不変条件）。
    pub oracle: String,
    /// 対象 semantic surface（`window`/`dock`/`theme` 等）。
    #[serde(default)]
    pub surfaces: Vec<String>,
    /// layer 別 binding。
    pub bindings: Vec<Binding>,
    /// evidence 出力先（repo 相対 path）。
    #[serde(default)]
    pub evidence: Option<String>,
}

/// 1 layer binding。status に応じて mechanism/owner/rerun が必須。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// layer 名。
    pub layer: String,
    /// verdict status。
    pub status: String,
    /// 実行 mechanism（`pass`/`fail` claim 時必須）。
    #[serde(default)]
    pub mechanism: Option<Mechanism>,
    /// 未実行時の Owner Issue URL（`not_run`/`blocked` 時必須）。
    #[serde(default)]
    pub owner: Option<String>,
    /// 再実行条件（`not_run`/`blocked`/`unsupported` 時必須）。
    #[serde(default)]
    pub rerun: Option<String>,
    /// binding 固有の補足。
    #[serde(default)]
    pub note: Option<String>,
}

/// 実行 mechanism。`cargo-test` は package+target(+filter)、
/// `mise-task` は task 名、`manual` は手順説明のみ。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mechanism {
    /// `cargo-test` / `mise-task` / `manual`。
    pub kind: String,
    /// `cargo-test` / `mise-task`（filter 検査時）の package 名。
    #[serde(default)]
    pub package: Option<String>,
    /// `cargo-test` の test target 名（tests/<target>.rs）。
    #[serde(default)]
    pub target: Option<String>,
    /// `cargo-test` の test name filter（`cargo test -- <filter>`）。
    /// `mise-task` では `package` 配下に実在を要求する test fn 名。
    #[serde(default)]
    pub filter: Option<String>,
    /// `mise-task` のタスク名。
    #[serde(default)]
    pub task: Option<String>,
    /// `manual` の手順説明。
    #[serde(default)]
    pub howto: Option<String>,
}

/// Native Component Lab story registry（`fixtures/driver-lab/stories.json`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryManifest {
    /// schemaVersion。
    #[serde(rename = "schemaVersion")]
    pub schema_version: u64,
    /// manifest 識別子（`rvf.driver-lab.stories/v1`）。
    pub manifest: String,
    /// 発行元 Issue URL。
    #[serde(default)]
    pub issue: Option<String>,
    /// Product Theme 宣言集合（story.theme はこの中から選ぶ —
    /// 2 Product Theme × 2 scale factor の matrix を registry 全体で
    /// 担保する）。
    pub themes: Vec<String>,
    /// story 一覧。
    pub stories: Vec<StoryEntry>,
}

/// 1 story。components・theme・scale factor・semantic states と
/// driver layer 別 binding を持つ。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryEntry {
    /// Story ID（`LAB-` prefix）。
    pub id: String,
    /// 件名。
    pub title: String,
    /// 対象 component 名。
    pub components: Vec<String>,
    /// Product Theme 名。
    pub theme: String,
    /// scale factor 一覧（最低 2 値）。
    #[serde(rename = "scaleFactor")]
    pub scale_factor: Vec<f64>,
    /// semantic state 一覧（focus/drag/commit/stale/invalid 等）。
    pub states: Vec<String>,
    /// driver layer 別 binding（layer → binding）。
    pub drivers: BTreeMap<String, Binding>,
    /// semantic oracle。
    pub oracle: String,
    /// evidence 出力先。
    #[serde(default)]
    pub evidence: Option<String>,
}

/// manifest 整合性違反。
#[derive(Debug)]
#[non_exhaustive]
pub struct ManifestError {
    /// 対象 id。
    pub subject: String,
    /// 違反内容。
    pub detail: String,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.subject, self.detail)
    }
}

/// cases.json を読み込む。
pub fn load_cases(path: &Path) -> Result<CaseManifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{} を読めません: {e}", path.display()))?;
    let manifest: CaseManifest = serde_json::from_str(&text)
        .map_err(|e| format!("{} の parse に失敗: {e}", path.display()))?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "schemaVersion {} は未対応",
            manifest.schema_version
        ));
    }
    if manifest.manifest != "rvf.driver-lab.cases/v1" {
        return Err(format!("manifest id が不正: {}", manifest.manifest));
    }
    Ok(manifest)
}

/// stories.json を読み込む。
pub fn load_stories(path: &Path) -> Result<StoryManifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{} を読めません: {e}", path.display()))?;
    let manifest: StoryManifest = serde_json::from_str(&text)
        .map_err(|e| format!("{} の parse に失敗: {e}", path.display()))?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "schemaVersion {} は未対応",
            manifest.schema_version
        ));
    }
    if manifest.manifest != "rvf.driver-lab.stories/v1" {
        return Err(format!("manifest id が不正: {}", manifest.manifest));
    }
    Ok(manifest)
}

/// binding の整合性を検査する（manifest/story 共通）。
fn validate_binding(
    subject: &str,
    layer: &str,
    binding: &Binding,
    errors: &mut Vec<ManifestError>,
) {
    let push = |detail: String, errors: &mut Vec<ManifestError>| {
        errors.push(ManifestError {
            subject: format!("{subject}/{layer}"),
            detail,
        });
    };
    if !VERDICT_VOCABULARY.contains(&binding.status.as_str()) {
        push(format!("status `{}` は語彙外", binding.status), errors);
        return;
    }
    match binding.status.as_str() {
        "pass" | "fail" => {
            let Some(m) = &binding.mechanism else {
                push("status=pass/fail は mechanism 必須".to_string(), errors);
                return;
            };
            match m.kind.as_str() {
                "cargo-test" => {
                    if m.package.is_none() || m.target.is_none() {
                        push(
                            "cargo-test mechanism は package+target 必須".to_string(),
                            errors,
                        );
                    }
                }
                "mise-task" => {
                    if m.task.is_none() {
                        push("mise-task mechanism は task 必須".to_string(), errors);
                    }
                }
                "manual" => {
                    if m.howto.is_none() {
                        push("manual mechanism は howto 必須".to_string(), errors);
                    }
                    if binding.status == "pass" && binding.note.is_none() {
                        push(
                            "manual+pass は検証記録の note 必須（not_run の \
                             成りすまし防止）"
                                .to_string(),
                            errors,
                        );
                    }
                }
                other => push(format!("mechanism kind `{other}` は未定義"), errors),
            }
        }
        "not_run" | "unsupported" | "blocked" => {
            if binding.owner.is_none() {
                push(
                    format!("status={} は owner(Issue URL) 必須", binding.status),
                    errors,
                );
            }
            if binding.rerun.is_none() {
                push(
                    format!("status={} は rerun 条件必須", binding.status),
                    errors,
                );
            }
        }
        _ => unreachable!("vocabulary checked"),
    }
}

/// case manifest の整合性を検査する。
#[must_use]
pub fn validate_cases(manifest: &CaseManifest) -> Vec<ManifestError> {
    let mut errors = Vec::new();
    for v in VERDICT_VOCABULARY {
        if !manifest.vocabulary.iter().any(|x| x == v) {
            errors.push(ManifestError {
                subject: "vocabulary".to_string(),
                detail: format!("`{v}` が vocabulary 宣言に無い"),
            });
        }
    }
    let mut ids = HashSet::new();
    for case in &manifest.cases {
        if !ids.insert(&case.id) {
            errors.push(ManifestError {
                subject: case.id.clone(),
                detail: "case id が重複".to_string(),
            });
        }
        // `UI-DVR-\d{2}` 形式（2 桁数字）。
        let digits = case.id.strip_prefix("UI-DVR-");
        if digits.is_none_or(|d| d.len() != 2 || !d.bytes().all(|b| b.is_ascii_digit())) {
            errors.push(ManifestError {
                subject: case.id.clone(),
                detail: "case id は `UI-DVR-NN`（2 桁数字）必須".to_string(),
            });
        }
        for binding in &case.bindings {
            if !manifest.layers.iter().any(|l| l == &binding.layer) {
                errors.push(ManifestError {
                    subject: format!("{}/{}", case.id, binding.layer),
                    detail: "layer が layers 宣言に無い".to_string(),
                });
            }
            validate_binding(&case.id, &binding.layer, binding, &mut errors);
        }
    }
    // requiredCases の完全性（manifest が欠落 case を通さないように pin）。
    for required in &manifest.required_cases {
        if !manifest.cases.iter().any(|c| &c.id == required) {
            errors.push(ManifestError {
                subject: required.clone(),
                detail: "requiredCases の case が manifest に存在しない".to_string(),
            });
        }
    }
    errors
}

/// story manifest の整合性を検査する。
#[must_use]
pub fn validate_stories(manifest: &StoryManifest) -> Vec<ManifestError> {
    let mut errors = Vec::new();
    let mut ids = HashSet::new();
    for story in &manifest.stories {
        if !ids.insert(&story.id) {
            errors.push(ManifestError {
                subject: story.id.clone(),
                detail: "story id が重複".to_string(),
            });
        }
        if story.scale_factor.len() < 2 {
            errors.push(ManifestError {
                subject: story.id.clone(),
                detail: "scaleFactor は 2 値以上必須".to_string(),
            });
        }
        if !manifest.themes.iter().any(|t| t == &story.theme) {
            errors.push(ManifestError {
                subject: story.id.clone(),
                detail: format!("theme `{}` は manifest.themes 宣言に無い", story.theme),
            });
        }
        for (layer, binding) in &story.drivers {
            if !LAYER_VOCABULARY.contains(&layer.as_str()) {
                errors.push(ManifestError {
                    subject: format!("{}/{layer}", story.id),
                    detail: "driver layer が語彙外".to_string(),
                });
            }
            validate_binding(&story.id, layer, binding, &mut errors);
        }
    }
    errors
}

/// case manifest の binding mechanism の存在確認（`cargo-test` /
/// `mise-task`）。`cargo-test` は package が metadata に存在し
/// tests/<target>.rs が実在すること、`filter` 指定時は test file source に
/// 同名 `fn` が存在することを検証する。
#[must_use]
pub fn verify_mechanisms(
    manifest: &CaseManifest,
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
) -> Vec<ManifestError> {
    manifest
        .cases
        .iter()
        .flat_map(|case| {
            case.bindings
                .iter()
                .map(move |b| (format!("{}/{}", case.id, b.layer), b))
        })
        .flat_map(|(subject, b)| verify_binding_mechanism(&subject, b, metadata, workspace_root))
        .collect()
}

/// story manifest の driver binding mechanism の存在確認（case 側と同一規則）。
#[must_use]
pub fn verify_story_mechanisms(
    manifest: &StoryManifest,
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
) -> Vec<ManifestError> {
    manifest
        .stories
        .iter()
        .flat_map(|story| {
            story
                .drivers
                .iter()
                .map(move |(layer, b)| (format!("{}/{layer}", story.id), b))
        })
        .flat_map(|(subject, b)| verify_binding_mechanism(&subject, b, metadata, workspace_root))
        .collect()
}

/// 1 binding の mechanism を実在確認する。`cargo-test` は package・
/// tests/<target>.rs・filter fn、`mise-task` は `mise.toml` の
/// `[tasks.<name>]` 定義を要求し、`package`+`filter` 指定時は package の
/// src/tests に同名 test fn を要求する（`manual` は手順説明のみで検査対象外）。
fn verify_binding_mechanism(
    subject: &str,
    binding: &Binding,
    metadata: &cargo_metadata::Metadata,
    workspace_root: &Path,
) -> Vec<ManifestError> {
    let subject = subject.to_owned();
    let error = |detail: String| ManifestError {
        subject: subject.clone(),
        detail,
    };
    let Some(m) = &binding.mechanism else {
        return Vec::new();
    };
    match m.kind.as_str() {
        "mise-task" => {
            let Some(task) = &m.task else {
                return Vec::new(); // schema 側で検出済み
            };
            let mise_toml =
                std::fs::read_to_string(workspace_root.join("mise.toml")).unwrap_or_default();
            let mut errors = Vec::new();
            if !mise_toml.contains(&format!("[tasks.{task}]")) {
                errors.push(error(format!(
                    "mise task `{task}` が mise.toml に定義されていない"
                )));
            }
            match (&m.package, &m.filter) {
                (Some(pkg_name), Some(filter)) => match package_manifest_dir(metadata, pkg_name) {
                    Some(dir) => {
                        if !["src", "tests"]
                            .iter()
                            .any(|d| package_has_test_fn(&dir.join(d), filter))
                        {
                            errors.push(error(format!(
                                    "package `{pkg_name}` の src/tests に filter `{filter}` に一致する test fn が見つからない"
                                )));
                        }
                    }
                    None => errors.push(error(format!("package `{pkg_name}` が metadata に無い"))),
                },
                (None, Some(_)) => errors.push(error(
                    "mise-task の filter は package 指定が必要".to_string(),
                )),
                _ => {}
            }
            errors
        }
        "cargo-test" => {
            let (Some(pkg_name), Some(target)) = (&m.package, &m.target) else {
                return Vec::new(); // schema 側で検出済み
            };
            let Some(manifest_dir) = package_manifest_dir(metadata, pkg_name) else {
                return vec![error(format!("package `{pkg_name}` が metadata に無い"))];
            };
            let test_file = manifest_dir.join("tests").join(format!("{target}.rs"));
            if !test_file.exists() {
                return vec![error(format!(
                    "tests/{target}.rs が存在しない（{}）",
                    test_file.display()
                ))];
            }
            let Some(filter) = &m.filter else {
                return Vec::new();
            };
            let src = std::fs::read_to_string(&test_file).unwrap_or_default();
            if has_test_fn(&src, filter) {
                Vec::new()
            } else {
                vec![error(format!(
                    "tests/{target}.rs に filter `{filter}` に一致する #[test] fn が見つからない"
                ))]
            }
        }
        _ => Vec::new(),
    }
}

fn package_manifest_dir(metadata: &cargo_metadata::Metadata, name: &str) -> Option<PathBuf> {
    let pkg = metadata.packages.iter().find(|p| p.name.as_str() == name)?;
    Some(PathBuf::from(
        pkg.manifest_path
            .parent()
            .map_or(".", cargo_metadata::camino::Utf8Path::as_str),
    ))
}

/// `cargo test` filter は substring match — filter を名前に含み、直前に
/// test 属性（`#[test]`/`#[tokio::test]`/`#[gpui::test]`）がある fn を要求する。
fn has_test_fn(src: &str, filter: &str) -> bool {
    let lines: Vec<&str> = src.lines().collect();
    lines.iter().enumerate().any(|(i, l)| {
        let t = l.trim_start();
        (t.starts_with("fn ") || t.starts_with("async fn "))
            && l.contains(filter)
            && lines[..i].iter().rev().take(6).any(|p| {
                p.contains("#[test]") || p.contains("#[tokio::test]") || p.contains("#[gpui::test]")
            })
    })
}

/// `dir` 配下（再帰）の `.rs` のいずれかに [`has_test_fn`] が成り立つか。
fn package_has_test_fn(dir: &Path, filter: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        let p = e.path();
        if p.is_dir() {
            package_has_test_fn(&p, filter)
        } else {
            p.extension().and_then(|x| x.to_str()) == Some("rs")
                && has_test_fn(&std::fs::read_to_string(&p).unwrap_or_default(), filter)
        }
    })
}

/// evidence JSONL の 1 行 schema。
#[derive(Serialize)]
struct EvidenceRow<'a> {
    manifest: &'static str,
    subject: &'a str,
    layer: &'a str,
    expected: &'a str,
    observed: &'a str,
    mechanism: &'a str,
    detail: &'a str,
}

/// evidence JSONL 行を `RUVIFORM_DRIVER_LAB_EVIDENCE` で指定された
/// path に append する。env 未設定では何もしない（cargo test を
/// 純粋に保つ）。相対 path は workspace root
/// （`CARGO_MANIFEST_DIR/../..`）から解決する — `cargo test` の
/// CWD は package root なので未解決の相対 path は crate 配下に
/// 漏れる。
///
/// `expected` は manifest 宣言 status、`observed` は実際に実行した
/// fixture の結果 — 乖離は呼び出し側が assertion にすること。
pub fn emit_evidence(
    subject: &str,
    layer: &str,
    expected: &str,
    observed: &str,
    mechanism: &str,
    detail: &str,
) {
    let Ok(raw) = std::env::var("RUVIFORM_DRIVER_LAB_EVIDENCE") else {
        return;
    };
    let path = {
        let p = PathBuf::from(&raw);
        if p.is_absolute() {
            p
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
        }
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let row = EvidenceRow {
        manifest: "rvf.driver-lab.evidence/v1",
        subject,
        layer,
        expected,
        observed,
        mechanism,
        detail,
    };
    if let Ok(line) = serde_json::to_string(&row) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}
