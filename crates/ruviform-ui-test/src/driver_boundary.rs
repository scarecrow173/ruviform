//! Issue #390 driver boundary guard — ruleset 型と 2 系統検査 engine。
//!
//! GUI toolkit / JS runtime / backend crate を protected package
//! （workspace member 全体・allowlist 例外のみ）から排除する検査を
//! 提供する。`cargo metadata` の依存グラフ検査と `syn` による
//! public signature 漏出検査を組み合わせ、grep 文字列一致では保証
//! できない境界を fail-closed で検査する。
//!
//! 検査の限界（rustdoc JSON 未使用・macro/生成コード未走査・glob
//! re-export の全展開なし）は `fixtures/driver-lab/boundary.json`
//! と test-spec に明記する。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use cargo_metadata::{Dependency, DependencyKind, Metadata, Package, PackageId};
use serde::{Deserialize, Serialize};

mod sig;

/// boundary ruleset（`fixtures/driver-lab/boundary.json`）の schema。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ruleset {
    /// schemaVersion（現在 1）。
    #[serde(rename = "schemaVersion")]
    pub schema_version: u64,
    /// ruleset 識別子（`rvf.driver-boundary/v1`）。
    pub ruleset: String,
    /// `true` の場合、`allowlist-grant-unused` も verdict を fail にする。
    #[serde(default)]
    pub strict: bool,
    /// 禁止 dep root 名。`prefix-*` 形式の接尾辞 glob を許可。
    #[serde(rename = "forbiddenDepRoots")]
    pub forbidden_dep_roots: Vec<String>,
    /// 禁止 path root ident（Rust ident 名、`-` は `_`）。
    #[serde(rename = "forbiddenPathRoots")]
    pub forbidden_path_roots: Vec<String>,
    /// allowlist（adapter/composition-root/extension のみ例外）。
    #[serde(default)]
    pub allowlist: Vec<AllowlistEntry>,
}

/// allowlist エントリ。明示 grant のみ禁止 dep 宣言を許可する。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowlistEntry {
    /// 対象 package 名。
    pub package: String,
    /// role: `driver-adapter` / `composition-root` / `driver-extension`。
    pub role: AllowlistRole,
    /// 宣言を許可する forbidden dep 名（全て `optional: true` 必須）。
    #[serde(default, rename = "allowedDeps")]
    pub allowed_deps: Vec<String>,
    /// 上記 dep の有効化を許可する feature 名。
    #[serde(default, rename = "viaFeatures")]
    pub via_features: Vec<String>,
    /// `true` の場合、package 不在を `allowlist-missing` violation にする。
    /// 未 merge driver（Issue #386 等）は `false` で着陸を待つ。
    #[serde(default)]
    pub required: bool,
    /// grant の根拠（Issue/ADR 参照）。
    #[serde(default)]
    pub reason: String,
}

/// allowlist role。例外の構造的意味を記録する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub enum AllowlistRole {
    /// `wry`/`gpui` 等 backend の driver adapter。
    #[serde(rename = "driver-adapter")]
    DriverAdapter,
    /// 実 binary/dylib を組み立てる composition root。
    #[serde(rename = "composition-root")]
    CompositionRoot,
    /// 隔離された driver extension。
    #[serde(rename = "driver-extension")]
    DriverExtension,
}

/// 1 検査の finding。code・package・位置・detail を持つ。
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Finding {
    /// 検査系統（`dep-graph` / `signature` / `allowlist`）。
    pub check: &'static str,
    /// finding code（`forbidden-dep-direct` 等）。
    pub code: &'static str,
    /// 対象 package 名。
    pub package: String,
    /// 位置（signature 検査のみ）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// 行番号（signature 検査のみ）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// detail（dep 名・witness path・item 名等）。
    pub detail: String,
}

/// 検査単位の report。
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct CheckReport {
    /// check id。
    pub id: &'static str,
    /// `pass` / `fail`。
    pub status: &'static str,
    /// findings。
    pub findings: Vec<Finding>,
}

/// guard 全体の verdict report。
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct GuardReport {
    /// schemaVersion。
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// report 識別子。
    pub report: &'static str,
    /// `pass` / `fail`。
    pub verdict: &'static str,
    /// check 別 report。
    pub checks: Vec<CheckReport>,
}

const CHECK_DEP_GRAPH: &str = "dep-graph";
const CHECK_SIGNATURE: &str = "signature";
const CHECK_ALLOWLIST: &str = "allowlist";

/// ruleset を読み込み、基本整合を検査する。
pub fn load_ruleset(path: &Path) -> Result<Ruleset, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("ruleset {} を読めません: {e}", path.display()))?;
    let ruleset: Ruleset = serde_json::from_str(&text)
        .map_err(|e| format!("ruleset {} の parse に失敗: {e}", path.display()))?;
    if ruleset.schema_version != 1 {
        return Err(format!(
            "ruleset schemaVersion {} は未対応（1 のみ）",
            ruleset.schema_version
        ));
    }
    if ruleset.ruleset != "rvf.driver-boundary/v1" {
        return Err(format!("ruleset id が不正: {}", ruleset.ruleset));
    }
    Ok(ruleset)
}

/// ruleset 内部整合 + allowlist 整合の findings。
#[must_use]
pub fn check_ruleset(ruleset: &Ruleset, metadata: &Metadata) -> Vec<Finding> {
    let mut findings = Vec::new();
    for grant in &ruleset.allowlist {
        for dep in &grant.allowed_deps {
            if !matches_forbidden(dep, &ruleset.forbidden_dep_roots) {
                findings.push(Finding {
                    check: CHECK_ALLOWLIST,
                    code: "allowlist-invalid-dep",
                    package: grant.package.clone(),
                    file: None,
                    line: None,
                    detail: format!("allowedDep `{dep}` は forbiddenDepRoots に一致しない"),
                });
            }
        }
        let present = metadata.packages.iter().any(|p| {
            metadata.workspace_members.contains(&p.id) && p.name.as_str() == grant.package
        });
        if grant.required && !present {
            findings.push(Finding {
                check: CHECK_ALLOWLIST,
                code: "allowlist-missing",
                package: grant.package.clone(),
                file: None,
                line: None,
                detail: format!(
                    "required grant `{}` の package が workspace に存在しない",
                    grant.package
                ),
            });
        }
        if present
            && let Some(pkg) = metadata
                .packages
                .iter()
                .find(|p| p.name.as_str() == grant.package)
        {
            for dep in &grant.allowed_deps {
                let declared = pkg.dependencies.iter().any(|d| {
                    d.name.as_str() == dep.as_str() || d.rename.as_deref() == Some(dep.as_str())
                });
                if !declared {
                    findings.push(Finding {
                        check: CHECK_ALLOWLIST,
                        code: "allowlist-grant-unused",
                        package: grant.package.clone(),
                        file: None,
                        line: None,
                        detail: format!(
                            "allowedDep `{dep}` が manifest に未宣言（grant 過剰の可能性）"
                        ),
                    });
                }
            }
        }
    }
    findings
}

/// 依存グラフ検査: direct dep / feature 有効化 / transitive・conditional
/// 到達の 3 観点で workspace member 全体を走査する。
#[must_use]
pub fn check_dep_graph(ruleset: &Ruleset, metadata: &Metadata) -> Vec<Finding> {
    let mut findings = Vec::new();
    let packages: HashMap<&PackageId, &Package> =
        metadata.packages.iter().map(|p| (&p.id, p)).collect();
    let nodes: HashMap<&PackageId, &cargo_metadata::Node> = metadata
        .resolve
        .as_ref()
        .map(|r| r.nodes.iter().map(|n| (&n.id, n)).collect())
        .unwrap_or_default();

    let allowlisted_ids: HashSet<PackageId> = metadata
        .workspace_members
        .iter()
        .filter_map(|id| packages.get(id))
        .filter(|p| {
            ruleset
                .allowlist
                .iter()
                .any(|g| g.package == p.name.as_str())
        })
        .map(|p| p.id.clone())
        .collect();

    for member_id in &metadata.workspace_members {
        let Some(pkg) = packages.get(member_id) else {
            continue;
        };
        let grant = ruleset
            .allowlist
            .iter()
            .find(|g| g.package == pkg.name.as_str());

        check_direct_deps(ruleset, pkg, grant, &mut findings);
        check_features(ruleset, pkg, grant, &mut findings);
        check_dep_activation(ruleset, metadata, &packages, pkg, grant, &mut findings);
        check_transitive(
            ruleset,
            pkg,
            &packages,
            &nodes,
            &allowlisted_ids,
            &mut findings,
        );
    }
    findings
}

fn check_direct_deps(
    ruleset: &Ruleset,
    pkg: &Package,
    grant: Option<&AllowlistEntry>,
    findings: &mut Vec<Finding>,
) {
    for dep in &pkg.dependencies {
        if !matches_forbidden(dep.name.as_str(), &ruleset.forbidden_dep_roots) {
            continue;
        }
        let kind = match dep.kind {
            DependencyKind::Normal => "normal",
            DependencyKind::Development => "dev",
            DependencyKind::Build => "build",
            DependencyKind::Unknown => "unknown",
        };
        match grant {
            Some(g) if g.allowed_deps.iter().any(|a| a == dep.name.as_str()) => {
                if !dep.optional {
                    findings.push(Finding {
                        check: CHECK_DEP_GRAPH,
                        code: "forbidden-dep-not-optional",
                        package: pkg.name.clone(),
                        file: None,
                        line: None,
                        detail: format!(
                            "allowlisted dep `{}` ({kind}) が optional でない",
                            dep.name
                        ),
                    });
                }
            }
            _ => findings.push(Finding {
                check: CHECK_DEP_GRAPH,
                code: "forbidden-dep-direct",
                package: pkg.name.clone(),
                file: None,
                line: None,
                detail: format!(
                    "forbidden dep `{}` ({kind}, optional={}) を宣言",
                    dep.name, dep.optional
                ),
            }),
        }
    }
}

/// feature 値の展開: `dep:X`・`X`（optional dep の暗黙 feature）・
/// feature→feature 連鎖を辿り、forbidden dep を有効化する feature を
/// `viaFeatures` 不在なら violation にする。
fn check_features(
    ruleset: &Ruleset,
    pkg: &Package,
    grant: Option<&AllowlistEntry>,
    findings: &mut Vec<Finding>,
) {
    let optional_deps: HashSet<&str> = pkg
        .dependencies
        .iter()
        .filter(|d| d.optional)
        .map(|d| d.rename.as_deref().unwrap_or(d.name.as_str()))
        .collect();
    let dep_name_by_feature: HashMap<&str, &str> = pkg
        .dependencies
        .iter()
        .map(|d| {
            (
                d.rename.as_deref().unwrap_or(d.name.as_str()),
                d.name.as_str(),
            )
        })
        .collect();

    for (fname, values) in &pkg.features {
        let mut enabled: BTreeSet<String> = BTreeSet::new();
        let mut stack: VecDeque<String> = values.iter().cloned().collect();
        let mut visited = HashSet::new();
        visited.insert(fname.clone());
        while let Some(v) = stack.pop_front() {
            if let Some(dep) = v.strip_prefix("dep:") {
                // `dep:` の operand は manifest の依存 key（rename
                // 後の名前）— 実 package 名へ解決して判定しないと
                // `dep:alias` で rename 迂回できる（H3）。
                let real = dep_name_by_feature.get(dep).copied().unwrap_or(dep);
                enabled.insert(real.to_string());
                continue;
            }
            if v.contains('/') {
                // `crate/feature` — 他 crate の feature 有効化。
                // forbidden root への dep 自体は direct-dep 検査が捕捉。
                continue;
            }
            if !visited.insert(v.clone()) {
                continue;
            }
            if let Some(next) = pkg.features.get(&v) {
                stack.extend(next.iter().cloned());
            }
            if let Some(dep) = dep_name_by_feature.get(v.as_str())
                && optional_deps.contains(v.as_str())
            {
                enabled.insert((*dep).to_string());
            }
        }
        for dep in enabled {
            if !matches_forbidden(&dep, &ruleset.forbidden_dep_roots) {
                continue;
            }
            let allowed = grant.is_some_and(|g| {
                g.allowed_deps.iter().any(|a| a == &dep)
                    && g.via_features.iter().any(|f| f == fname)
            });
            if !allowed {
                findings.push(Finding {
                    check: CHECK_DEP_GRAPH,
                    code: "forbidden-dep-feature",
                    package: pkg.name.clone(),
                    file: None,
                    line: None,
                    detail: format!("feature `{fname}` が forbidden dep `{dep}` を有効化"),
                });
            }
        }
    }
}

/// dep 宣言側の feature 有効化検査。`{ features = ["webview"] }`
/// 形式は features 表を経由せず adapter の toolkit feature を直接
/// 有効化するため、個別検査が必要（RB-01）。
///
/// 加えて、allowlisted adapter package への依存自体を
/// composition-root/driver-extension grant なしの member に禁じる
/// （`adapter-consumer`）。spec R8 の「adapter への依存は
/// composition root のみ」規則。
fn check_dep_activation(
    ruleset: &Ruleset,
    metadata: &Metadata,
    packages: &HashMap<&PackageId, &Package>,
    pkg: &Package,
    grant: Option<&AllowlistEntry>,
    findings: &mut Vec<Finding>,
) {
    let consumer_ok = grant.is_some_and(|g| {
        matches!(
            g.role,
            AllowlistRole::CompositionRoot | AllowlistRole::DriverExtension
        )
    });
    for dep in &pkg.dependencies {
        // 依存先 package が allowlisted adapter か調べる。resolve
        // node の辺を優先し、resolve に現れない未 activation の
        // optional 辺は workspace member の name 一致で解決する —
        // optional/dev 宣言だけでは adapter 依存の検査をすり抜け
        // られない（H1/H2: kind・activation 状態を不問とする）。
        let target_id = metadata
            .resolve
            .as_ref()
            .and_then(|r| {
                r.nodes.iter().find(|n| n.id == pkg.id).and_then(|n| {
                    n.deps
                        .iter()
                        .find(|d| {
                            packages
                                .get(&d.pkg)
                                .is_some_and(|p| p.name.as_str() == dep.name.as_str())
                        })
                        .map(|d| d.pkg.clone())
                })
            })
            .or_else(|| {
                metadata
                    .workspace_members
                    .iter()
                    .filter_map(|id| packages.get(id))
                    .find(|p| p.name.as_str() == dep.name.as_str())
                    .map(|p| p.id.clone())
            });
        let Some(target_id) = target_id else {
            continue;
        };
        let Some(target_pkg) = packages.get(&target_id) else {
            continue;
        };
        let Some(target_grant) = ruleset
            .allowlist
            .iter()
            .find(|g| g.package == target_pkg.name.as_str())
        else {
            continue;
        };
        if consumer_ok {
            // composition-root / driver-extension は adapter への依存と
            // toolkit feature の有効化自体が職務 — 以降の検査は不要。
            continue;
        }
        let kind = match dep.kind {
            DependencyKind::Normal => "normal",
            DependencyKind::Development => "dev",
            DependencyKind::Build => "build",
            DependencyKind::Unknown => "unknown",
        };
        findings.push(Finding {
            check: CHECK_DEP_GRAPH,
            code: "adapter-consumer",
            package: pkg.name.clone(),
            file: None,
            line: None,
            detail: format!(
                "allowlisted adapter `{}` への依存（{kind}, optional={}）は \
                 composition-root grant なしでは不可",
                target_pkg.name, dep.optional
            ),
        });
        // consumer 側 feature 表の `depkey/f`・`depkey?/f` 値が
        // adapter の viaFeatures を起動する経路 — optional dep +
        // feature 有効化でも検出する（H1: `evil = ["dep:a",
        // "a/gpui"]` 形の迂回を塞ぐ）。
        let depkey = dep.rename.as_deref().unwrap_or(dep.name.as_str());
        feature_table_activation(pkg, depkey, target_pkg, target_grant, findings);
        dep_feature_activation(dep, pkg, target_pkg, target_grant, findings);
    }
}

/// consumer feature 表の `<depkey>/<f>`・`<depkey>?/<f>` 値が
/// adapter の viaFeatures を起動する経路を検査する。
fn feature_table_activation(
    pkg: &Package,
    depkey: &str,
    target_pkg: &Package,
    target_grant: &AllowlistEntry,
    findings: &mut Vec<Finding>,
) {
    for (fname, values) in &pkg.features {
        for v in values {
            let Some(tail) = v
                .strip_prefix(depkey)
                .and_then(|r| r.strip_prefix('/').or_else(|| r.strip_prefix("?/")))
            else {
                continue;
            };
            if target_grant.via_features.iter().any(|vf| vf == tail) {
                findings.push(Finding {
                    check: CHECK_DEP_GRAPH,
                    code: "forbidden-dep-adapter-feature",
                    package: pkg.name.clone(),
                    file: None,
                    line: None,
                    detail: format!(
                        "feature `{fname}` の値 `{v}` が adapter `{}` の \
                         toolkit feature `{tail}` を起動",
                        target_pkg.name
                    ),
                });
            }
        }
    }
}

/// dep 宣言の `features`/`uses_default_features` が adapter の
/// viaFeatures を起動する経路を検査する（RB-01）。adapter 側の
/// feature→feature 連鎖も辿る。
fn dep_feature_activation(
    dep: &Dependency,
    pkg: &Package,
    target_pkg: &Package,
    target_grant: &AllowlistEntry,
    findings: &mut Vec<Finding>,
) {
    let mut activated: BTreeSet<String> = dep.features.iter().cloned().collect();
    if dep.uses_default_features
        && let Some(default) = target_pkg.features.get("default")
    {
        activated.extend(default.iter().cloned());
    }
    let mut work: VecDeque<String> = activated.iter().cloned().collect();
    let mut seen: BTreeSet<String> = activated;
    let mut hits: Vec<String> = Vec::new();
    while let Some(f) = work.pop_front() {
        if target_grant.via_features.iter().any(|v| v == &f) {
            hits.push(f.clone());
        }
        // dep:X 形式は dep 直接有効化 — feature 名としては辿れない。
        if let Some(vals) = target_pkg.features.get(&f) {
            for v in vals {
                if !v.contains('/') && !v.starts_with("dep:") && seen.insert(v.clone()) {
                    work.push_back(v.clone());
                }
            }
        }
    }
    for hit in hits {
        findings.push(Finding {
            check: CHECK_DEP_GRAPH,
            code: "forbidden-dep-adapter-feature",
            package: pkg.name.clone(),
            file: None,
            line: None,
            detail: format!(
                "dep `{}` の feature 有効化が adapter `{}` の toolkit \
                 feature `{hit}` を起動",
                dep.name, target_pkg.name
            ),
        });
    }
}

/// transitive 到達検査。起点 member からは normal+build+dev、
/// それ以降は normal+build の非 optional 辺のみ辿る（dev-dep は
/// cargo 意味論上推移しない）。optional 辺込みの到達は別途
/// `forbidden-dep-conditional` として記録する。
/// allowlisted adapter の辺は展開しない（その optional toolkit 辺は
/// grant が許可する san ctioned channel であり、protected crate の
/// 到達判定に含めない）。
fn check_transitive(
    ruleset: &Ruleset,
    root: &Package,
    packages: &HashMap<&PackageId, &Package>,
    nodes: &HashMap<&PackageId, &cargo_metadata::Node>,
    allowlisted_ids: &HashSet<PackageId>,
    findings: &mut Vec<Finding>,
) {
    // 起点が allowlisted adapter の場合、grant が許可する dep への
    // 辺は sanctioned channel（optional かどうかは direct-dep 検査が
    // 別途担保）のため到達判定から除く。
    let root_exempt: HashSet<String> = ruleset
        .allowlist
        .iter()
        .find(|g| g.package == root.name.as_str())
        .map(|g| g.allowed_deps.iter().cloned().collect())
        .unwrap_or_default();
    let mandatory = reach_set(
        root,
        packages,
        nodes,
        allowlisted_ids,
        &root_exempt,
        &ruleset.forbidden_dep_roots,
    );
    let conditional = conditional_reach(
        root,
        packages,
        nodes,
        allowlisted_ids,
        &root_exempt,
        &ruleset.forbidden_dep_roots,
        findings,
    );
    for (name, path) in &mandatory {
        findings.push(Finding {
            check: CHECK_DEP_GRAPH,
            code: "forbidden-dep-transitive",
            package: root.name.clone(),
            file: None,
            line: None,
            detail: format!("forbidden dep `{name}` に非 optional 辺で到達: {path}"),
        });
    }
    for (name, path) in &conditional {
        if mandatory.contains_key(name) {
            continue;
        }
        findings.push(Finding {
            check: CHECK_DEP_GRAPH,
            code: "forbidden-dep-conditional",
            package: root.name.clone(),
            file: None,
            line: None,
            detail: format!("forbidden dep `{name}` に optional 辺経由で到達可能: {path}"),
        });
    }
}

/// root package からの forbidden 無条件到達集合を返す
/// （package 名 → witness path）。`cargo metadata` の resolve graph
/// は未 activation の optional dep を含まないため、resolve に現れる
/// 辺は全て activation 済み = 無条件到達として扱う。
/// 起点からは dev 辺も辿る（自身の test lane も保護対象）。下流
/// node からは normal/build のみ（dev-dep は cargo 意味論上推移しない）。
fn reach_set(
    root: &Package,
    packages: &HashMap<&PackageId, &Package>,
    nodes: &HashMap<&PackageId, &cargo_metadata::Node>,
    allowlisted_ids: &HashSet<PackageId>,
    root_exempt: &HashSet<String>,
    forbidden: &[String],
) -> BTreeMap<String, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut queue: VecDeque<(PackageId, String)> = VecDeque::new();
    let mut visited: HashSet<PackageId> = HashSet::new();
    queue.push_back((root.id.clone(), root.name.clone()));
    visited.insert(root.id.clone());
    while let Some((pid, path)) = queue.pop_front() {
        let is_root = pid == root.id;
        if !packages.contains_key(&pid) {
            continue;
        }
        let Some(node) = nodes.get(&pid) else {
            continue;
        };
        for dep in &node.deps {
            // allowlisted adapter の outgoing 辺は展開しない —
            // adapter の optional toolkit 辺は grant が許可する
            // sanctioned channel であり、到達判定の対象外。
            if allowlisted_ids.contains(&dep.pkg) {
                continue;
            }
            let Some(target_pkg) = packages.get(&dep.pkg).copied() else {
                continue;
            };
            // allowlisted root の grant 済み dep 辺は到達判定から除外。
            if is_root && root_exempt.contains(target_pkg.name.as_str()) {
                continue;
            }
            // 下流 node からは dev-dep を辿らない。
            let traversable = dep.dep_kinds.is_empty()
                || dep
                    .dep_kinds
                    .iter()
                    .any(|dk| is_root || dk.kind != DependencyKind::Development);
            if !traversable {
                continue;
            }
            let next_path = format!("{path} -> {}", target_pkg.name);
            if matches_forbidden(target_pkg.name.as_str(), forbidden) {
                found.entry(target_pkg.name.clone()).or_insert(next_path);
                continue;
            }
            if visited.insert(dep.pkg.clone()) {
                queue.push_back((dep.pkg.clone(), next_path));
            }
        }
    }
    found
}

/// `conditional_reach` の走査単位 — resolve node または
/// manifest-only package（manifest dir 識別）。
#[derive(Clone)]
enum ReachUnit {
    Resolve(PackageId),
    Manifest(std::path::PathBuf),
}

struct ReachItem {
    unit: ReachUnit,
    path: String,
    key: String,
    /// 経路が optional 辺を 1 本以上含むか。
    cond: bool,
}

/// unit の manifest 依存宣言と resolve 無条件辺を集める。
/// Manifest-only unit は `cargo metadata --no-deps --offline` で
/// manifest を読み依存宣言のみ返す（resolve 辺なし）。解決失敗は
/// `conditional-manifest-error` finding で記録する — 無言 skip は
/// fail-open となり「到達不能」の無根拠な保証になるため。
fn unit_deps(
    unit: &ReachUnit,
    packages: &HashMap<&PackageId, &Package>,
    nodes: &HashMap<&PackageId, &cargo_metadata::Node>,
    is_root: bool,
    manifest_pkgs: &mut HashMap<std::path::PathBuf, Option<Vec<Dependency>>>,
    root_name: &str,
    findings: &mut Vec<Finding>,
) -> Option<(Vec<Dependency>, Vec<PackageId>)> {
    match unit {
        ReachUnit::Resolve(pid) => {
            let pkg = packages.get(pid)?;
            let edges = nodes
                .get(pid)
                .map(|n| {
                    n.deps
                        .iter()
                        .filter(|d| {
                            d.dep_kinds.is_empty()
                                || d.dep_kinds
                                    .iter()
                                    .any(|dk| is_root || dk.kind != DependencyKind::Development)
                        })
                        .map(|d| d.pkg.clone())
                        .collect()
                })
                .unwrap_or_default();
            Some((pkg.dependencies.clone(), edges))
        }
        ReachUnit::Manifest(dir) => {
            let mdeps = manifest_pkgs
                .entry(dir.clone())
                .or_insert_with(|| manifest_deps(dir))
                .clone();
            if let Some(d) = mdeps {
                return Some((d, Vec::new()));
            }
            findings.push(Finding {
                check: CHECK_DEP_GRAPH,
                code: "conditional-manifest-error",
                package: root_name.to_string(),
                file: None,
                line: None,
                detail: format!(
                    "manifest 走査で `{}` の deps を解決不能 \
                     （fail-closed として検査不能を記録）",
                    dir.display()
                ),
            });
            None
        }
    }
}

/// manifest dir の package 依存宣言を `--no-deps` で読む。
/// virtual/multi-member manifest では `manifest_path` が一致する
/// package を選び、一意特定できない場合は `None`（曖昧な package
/// 選択による誤走査を避ける）。
fn manifest_deps(dir: &Path) -> Option<Vec<Dependency>> {
    let want = dir.join("Cargo.toml");
    let want = want.canonicalize().unwrap_or(want);
    let md = cargo_metadata::MetadataCommand::new()
        .manifest_path(&want)
        .no_deps()
        .other_options(vec!["--offline".to_string()])
        .exec()
        .ok()?;
    md.packages
        .iter()
        .find(|p| {
            let m = p.manifest_path.as_std_path().to_path_buf();
            m.canonicalize().unwrap_or(m) == want
        })
        .map(|p| p.dependencies.clone())
}

/// manifest dep の target を解決する — workspace package name 一致
/// 優先、なければ `dep.path` の manifest dir。registry dep の未
/// activation target は解決不能で `None`（既知の限界）。
fn mdep_target(
    unit: &ReachUnit,
    mdep: &Dependency,
    packages: &HashMap<&PackageId, &Package>,
    resolve_edge_set: &HashSet<PackageId>,
    is_root: bool,
    root_exempt: &HashSet<String>,
    allowlisted_ids: &HashSet<PackageId>,
) -> Option<(ReachUnit, String, String)> {
    if let Some(tp) = packages
        .values()
        .find(|p| p.name.as_str() == mdep.name.as_str())
    {
        let blocked = resolve_edge_set.contains(&tp.id)
            || (is_root && root_exempt.contains(tp.name.as_str()))
            || allowlisted_ids.contains(&tp.id);
        if blocked {
            return None;
        }
        return Some((
            ReachUnit::Resolve(tp.id.clone()),
            tp.name.clone(),
            format!("r:{}", tp.id),
        ));
    }
    let rel = mdep.path.as_ref()?;
    let base = match unit {
        ReachUnit::Manifest(dir) => dir.clone(),
        ReachUnit::Resolve(pid) => packages
            .get(pid)
            .map(|p| {
                p.manifest_path
                    .as_std_path()
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_path_buf()
            })
            .unwrap_or_default(),
    };
    let dir = base.join(rel.as_std_path());
    Some((
        ReachUnit::Manifest(dir.clone()),
        mdep.name.clone(),
        format!("m:{}", dir.display()),
    ))
}

/// `optional = true` 辺を 1 本以上含む経路での forbidden 到達集合を
/// 返す（package 名 → witness path）。resolve graph は未 activation
/// の optional dep を省略するため、optional 辺の target は
/// manifest 宣言から解決する — workspace packages で name 一致、
/// なければ `dep.path` 先の manifest を再帰走査する。
fn conditional_reach(
    root: &Package,
    packages: &HashMap<&PackageId, &Package>,
    nodes: &HashMap<&PackageId, &cargo_metadata::Node>,
    allowlisted_ids: &HashSet<PackageId>,
    root_exempt: &HashSet<String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) -> BTreeMap<String, String> {
    let mut manifest_pkgs: HashMap<std::path::PathBuf, Option<Vec<Dependency>>> = HashMap::new();
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut queue: VecDeque<ReachItem> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let root_key = format!("r:{}", root.id);
    queue.push_back(ReachItem {
        unit: ReachUnit::Resolve(root.id.clone()),
        path: root.name.clone(),
        key: root_key.clone(),
        cond: false,
    });
    visited.insert(root_key.clone());

    while let Some(item) = queue.pop_front() {
        let is_root = item.key == root_key;
        let Some((mdeps, resolve_edges)) = unit_deps(
            &item.unit,
            packages,
            nodes,
            is_root,
            &mut manifest_pkgs,
            root.name.as_str(),
            findings,
        ) else {
            continue;
        };
        let resolve_edge_set: HashSet<PackageId> = resolve_edges.iter().cloned().collect();
        let mut push = |tunit: ReachUnit,
                        tname: &str,
                        tkey: String,
                        cond: bool,
                        found: &mut BTreeMap<String, String>,
                        queue: &mut VecDeque<ReachItem>| {
            let next_path = format!("{} -> {tname}", item.path);
            if matches_forbidden(tname, forbidden) {
                if cond {
                    found.entry(tname.to_string()).or_insert(next_path);
                }
                return;
            }
            if visited.insert(tkey.clone()) {
                queue.push_back(ReachItem {
                    unit: tunit,
                    path: next_path,
                    key: tkey,
                    cond,
                });
            }
        };

        // resolve 辺 — 無条件。辺自体は無条件だが、経路が既に
        // conditional なら見つけた forbidden も conditional。
        for tid in &resolve_edges {
            if allowlisted_ids.contains(tid) {
                continue;
            }
            let Some(tp) = packages.get(tid).copied() else {
                continue;
            };
            if is_root && root_exempt.contains(tp.name.as_str()) {
                continue;
            }
            push(
                ReachUnit::Resolve(tid.clone()),
                tp.name.as_str(),
                format!("r:{tid}"),
                item.cond,
                &mut found,
                &mut queue,
            );
        }

        // manifest 宣言辺 — Resolve unit は未 activation の optional
        // 辺のみ（起点は自身 dev 辺も対象）、Manifest unit は非 dev
        // の全辺を辿る。
        for mdep in &mdeps {
            let edge_optional = mdep.optional;
            let skip = match &item.unit {
                ReachUnit::Resolve(_) => {
                    !edge_optional || (!is_root && mdep.kind == DependencyKind::Development)
                }
                ReachUnit::Manifest(_) => mdep.kind == DependencyKind::Development,
            };
            if skip {
                continue;
            }
            let Some((tunit, tname, tkey)) = mdep_target(
                &item.unit,
                mdep,
                packages,
                &resolve_edge_set,
                is_root,
                root_exempt,
                allowlisted_ids,
            ) else {
                continue;
            };
            push(
                tunit,
                &tname,
                tkey,
                item.cond || edge_optional,
                &mut found,
                &mut queue,
            );
        }
    }
    found
}

/// `x`/`prefix-*` パターンに名前が一致するか。
#[must_use]
pub fn matches_forbidden(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        if let Some(prefix) = p.strip_suffix('*') {
            name.starts_with(prefix)
        } else {
            name == p
        }
    })
}

/// public signature 漏出検査（`sig` module へ委譲）。全 workspace
/// member の lib target を走査する — allowlisted adapter も対象で、
/// adapter の公開 API に backend 型が出ること自体を漏出として検出
/// する（private 利用は `pub` item でないため検出しない）。
#[must_use]
pub fn check_signatures(ruleset: &Ruleset, metadata: &Metadata) -> Vec<Finding> {
    sig::check(metadata, &ruleset.forbidden_path_roots)
}

/// 全検査を実行し verdict report を返す。
#[must_use]
pub fn run(ruleset: &Ruleset, metadata: &Metadata) -> GuardReport {
    let allowlist_findings = check_ruleset(ruleset, metadata);
    let dep_findings = check_dep_graph(ruleset, metadata);
    let sig_findings = check_signatures(ruleset, metadata);

    let allowlist_fail = allowlist_findings
        .iter()
        .any(|f| ruleset.strict || f.code != "allowlist-grant-unused");
    let verdict = if allowlist_fail || !dep_findings.is_empty() || !sig_findings.is_empty() {
        "fail"
    } else {
        "pass"
    };
    GuardReport {
        schema_version: 1,
        report: "rvf.driver-boundary/v1",
        verdict,
        checks: vec![
            CheckReport {
                id: CHECK_DEP_GRAPH,
                status: if dep_findings.is_empty() {
                    "pass"
                } else {
                    "fail"
                },
                findings: dep_findings,
            },
            CheckReport {
                id: CHECK_SIGNATURE,
                status: if sig_findings.is_empty() {
                    "pass"
                } else {
                    "fail"
                },
                findings: sig_findings,
            },
            CheckReport {
                id: CHECK_ALLOWLIST,
                status: if allowlist_findings.is_empty() {
                    "pass"
                } else if allowlist_fail {
                    "fail"
                } else {
                    "warn"
                },
                findings: allowlist_findings,
            },
        ],
    }
}
